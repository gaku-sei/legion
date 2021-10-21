use std::{
    any::Any,
    cell::{Cell, UnsafeCell},
    collections::HashMap,
    io,
    ops::{Deref, DerefMut},
    path::Path,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use legion_content_store::ContentStore;

use crate::{
    asset_loader::{create_loader, AssetLoaderStub, LoaderResult},
    manifest::Manifest,
    vfs, Asset, AssetLoader, Handle, HandleUntyped, Resource, ResourceId, ResourceType,
};

/// Wraps a borrowed reference to a resource.
///
/// This wrapper type helps track the number of references to resources.
/// For more see [`AssetRegistry`].
pub struct Ref<'a, T> {
    resource: &'a T,
    guard: &'a AtomicIsize,
}

impl<'a, T> Drop for Ref<'a, T> {
    fn drop(&mut self) {
        assert!(0 < self.guard.fetch_sub(1, Ordering::Acquire));
    }
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = &'a T;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<'a, T> Ref<'a, T> {
    fn new(resource: &'a T, guard: &'a AtomicIsize) -> Self {
        assert!(0 <= guard.fetch_add(1, Ordering::Acquire));
        Self { resource, guard }
    }
}

struct InnerReadGuard<'a> {
    inner: &'a UnsafeCell<Inner>,
    guard: &'a AtomicIsize,
}

impl<'a> InnerReadGuard<'a> {
    fn new(inner: &'a UnsafeCell<Inner>, guard: &'a AtomicIsize) -> Self {
        assert!(0 <= guard.fetch_add(1, Ordering::Acquire));
        Self { inner, guard }
    }

    fn detach<'b>(&self) -> &'b Inner {
        unsafe { self.inner.get().as_ref().unwrap() }
    }
}

impl<'a> Drop for InnerReadGuard<'a> {
    fn drop(&mut self) {
        assert!(0 < self.guard.fetch_sub(1, Ordering::Acquire));
    }
}

impl<'a> Deref for InnerReadGuard<'a> {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.get().as_ref().unwrap() }
    }
}

struct InnerWriteGuard<'a> {
    inner: &'a UnsafeCell<Inner>,
    guard: &'a AtomicIsize,
}

impl<'a> InnerWriteGuard<'a> {
    fn new(inner: &'a UnsafeCell<Inner>, guard: &'a AtomicIsize) -> Self {
        assert!(0 == guard.fetch_sub(1, Ordering::Acquire));
        Self { inner, guard }
    }
}

impl<'a> Drop for InnerWriteGuard<'a> {
    fn drop(&mut self) {
        assert!(-1 == self.guard.fetch_add(1, Ordering::Acquire));
    }
}

impl<'a> Deref for InnerWriteGuard<'a> {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.get().as_ref().unwrap() }
    }
}

impl<'a> DerefMut for InnerWriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.get().as_mut().unwrap() }
    }
}

/// Options which can be used to configure the creation of [`AssetRegistry`].
pub struct AssetRegistryOptions {
    loaders: HashMap<ResourceType, Box<dyn AssetLoader + Send>>,
    devices: Vec<Box<dyn vfs::Device>>,
}

impl AssetRegistryOptions {
    /// Creates a blank set of options for [`AssetRegistry`] configuration.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            devices: vec![],
        }
    }

    /// Specifying `directory device` will mount a device that allows to read resources
    /// from a specified directory.
    pub fn add_device_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.devices.push(Box::new(vfs::DirDevice::new(path)));
        self
    }

    /// Specifying `content-addressable storage device` will mount a device that allows
    /// to read resources from a specified content store through provided manifest.
    pub fn add_device_cas(
        mut self,
        content_store: Box<dyn ContentStore>,
        manifest: Manifest,
    ) -> Self {
        self.devices
            .push(Box::new(vfs::CasDevice::new(manifest, content_store)));
        self
    }

    /// Enables support of a given [`Resource`] by adding corresponding [`AssetLoader`].
    pub fn add_loader<A: Asset>(mut self) -> Self {
        self.loaders.insert(A::TYPE, Box::new(A::Loader::default()));
        self
    }

    /// Creates [`AssetRegistry`] based on `AssetRegistryOptions`.
    pub fn create(self) -> Arc<AssetRegistry> {
        let (loader, mut io) = create_loader(self.devices);

        let registry = Arc::new(AssetRegistry {
            rw_guard: AtomicIsize::new(0),
            inner: UnsafeCell::new(Inner {
                assets: HashMap::new(),
                load_errors: HashMap::new(),
                load_event_senders: Vec::new(),
            }),
            loader,
            load_thread: Cell::new(None),
        });

        for (kind, mut loader) in self.loaders {
            loader.register_registry(Arc::clone(&registry));
            io.register_loader(kind, loader);
        }

        let load_thread = thread::spawn(move || {
            let mut loader = io;
            while loader.wait(Duration::from_millis(100)).is_some() {}
        });

        registry.load_thread.set(Some(load_thread));

        registry
    }
}

struct Inner {
    assets: HashMap<ResourceId, Box<dyn Any + Send + Sync>>,
    load_errors: HashMap<ResourceId, io::ErrorKind>,
    load_event_senders: Vec<crossbeam_channel::Sender<ResourceLoadEvent>>,
}

/// Registry of all loaded [`Resource`]s.
///
/// Provides an API to load assets by their [`ResourceId`]. The lifetime of an [`Resource`] is determined
/// by the reference counted [`HandleUntyped`] and [`Handle`].
///
/// # Safety:
///
/// The `update` method can only be called when no outstanding references `Ref` to resources exist.
/// No other method can be called concurrently with `update` method.
///
/// [`Handle`]: [`crate::Handle`]
pub struct AssetRegistry {
    rw_guard: AtomicIsize,
    inner: UnsafeCell<Inner>,
    loader: AssetLoaderStub,
    load_thread: Cell<Option<JoinHandle<()>>>,
}

/// A resource loading event is emitted when a resource is loaded, unloaded, or loading fails
#[derive(Clone, Copy)]
pub enum ResourceLoadEvent {
    /// Successful resource load, resulting from either a handle load, or the loading of a dependency
    Loaded(ResourceId),
    /// Resource unload event
    Unloaded(ResourceId),
    /// Sent when a loading attempt has failed
    LoadError(ResourceId),
}

impl Drop for AssetRegistry {
    fn drop(&mut self) {
        self.loader.terminate();
        self.load_thread.take().unwrap().join().unwrap();
    }
}

/// Safety: it is safe share references to `AssetRegistry` between threads
/// and the implementation will panic! if its safety rules are not fulfilled.
unsafe impl Sync for AssetRegistry {}

impl AssetRegistry {
    fn read_inner(&self) -> InnerReadGuard<'_> {
        InnerReadGuard::new(&self.inner, &self.rw_guard)
    }

    fn write_inner(&self) -> InnerWriteGuard<'_> {
        InnerWriteGuard::new(&self.inner, &self.rw_guard)
    }

    /// Requests an asset load.
    ///
    /// The asset will be unloaded after all instances of [`HandleUntyped`] and
    /// [`Handle`] that refer to that asset go out of scope.
    pub fn load_untyped(&self, id: ResourceId) -> HandleUntyped {
        self.loader.load(id)
    }

    /// Trigger a reload of a given primary resource.
    pub fn reload(&self, id: ResourceId) -> bool {
        self.loader.reload(id)
    }

    /// Returns a handle to the resource if a handle to this resource already exists.
    pub fn get_untyped(&self, id: ResourceId) -> Option<HandleUntyped> {
        self.loader.get_handle(id)
    }

    /// Returns a handle to the resource.
    /// If a handle to this resource does not already exist, a new one will be created.
    pub fn get_or_create_untyped(&self, id: ResourceId) -> HandleUntyped {
        self.loader.get_or_create_handle(id)
    }

    /// Same as [`Self::load_untyped`] but blocks until the resource load completes or returns an error.
    pub fn load_untyped_sync(&self, id: ResourceId) -> HandleUntyped {
        let handle = self.loader.load(id);
        // todo: this will be improved with async/await
        while !handle.is_loaded(self) && !handle.is_err(self) {
            self.update();
            std::thread::sleep(Duration::from_micros(100));
        }

        handle
    }

    /// Same as [`Self::load_untyped`] but the returned handle is generic over asset type `T` for convenience.
    pub fn load<T: Any + Resource>(&self, id: ResourceId) -> Handle<T> {
        let handle = self.load_untyped(id);
        Handle::<T>::from(handle)
    }

    /// Same as [`Self::load`] but blocks until the resource load completes or returns an error.
    pub fn load_sync<T: Any + Resource>(&self, id: ResourceId) -> Handle<T> {
        let handle = self.load_untyped_sync(id);
        Handle::<T>::from(handle)
    }

    /// Retrieves a reference to an asset, None if asset is not loaded.
    pub(crate) fn get<T: Any + Resource>(&self, id: ResourceId) -> Option<Ref<'_, T>> {
        let inner = self.read_inner();

        if let Some(asset) = inner.detach().assets.get(&id) {
            return asset
                .downcast_ref::<T>()
                .map(|a| Ref::new(a, &self.rw_guard));
        }
        None
    }

    /// Tests if an asset is loaded.
    pub(crate) fn is_loaded(&self, id: ResourceId) -> bool {
        self.read_inner().assets.get(&id).is_some()
    }

    /// Unloads assets based on their reference counts.
    pub fn update(&self) {
        let mut load_events = Vec::new();

        {
            let mut inner = self.write_inner();
            for removed_id in self.loader.collect_dropped_handles() {
                inner.load_errors.remove(&removed_id);
                inner.assets.remove(&removed_id);
                self.loader.unload(removed_id);
            }

            while let Some(result) = self.loader.try_result() {
                // todo: add success/failure callbacks using the provided LoadId.
                match result {
                    LoaderResult::Loaded(asset_id, asset, _load_id) => {
                        inner.assets.insert(asset_id, asset);
                        load_events.push(ResourceLoadEvent::Loaded(asset_id));
                    }
                    LoaderResult::Unloaded(asset_id) => {
                        inner.assets.remove(&asset_id);
                        load_events.push(ResourceLoadEvent::Unloaded(asset_id));
                    }
                    LoaderResult::LoadError(asset_id, _load_id, error_kind) => {
                        inner.load_errors.insert(asset_id, error_kind);
                        load_events.push(ResourceLoadEvent::LoadError(asset_id));
                    }
                }
            }
        }

        {
            // broadcast load events
            let inner = self.read_inner();
            for sender in &inner.load_event_senders {
                for event in &load_events {
                    sender.send(*event).unwrap();
                }
            }
        }
    }

    pub(crate) fn is_err(&self, id: ResourceId) -> bool {
        self.read_inner().load_errors.contains_key(&id)
    }

    /// Subscribe to load events, to know when resources are loaded and unloaded.
    /// Returns a channel receiver that will receive `ResourceLoadEvent`s.
    pub fn subscribe_to_load_events(&self) -> crossbeam_channel::Receiver<ResourceLoadEvent> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.write_inner().load_event_senders.push(sender);
        receiver
    }
}

#[cfg(test)]
mod tests {

    use legion_content_store::RamContentStore;

    use crate::test_asset;

    use super::*;

    fn setup_singular_asset_test(content: &[u8]) -> (ResourceId, Arc<AssetRegistry>) {
        let mut content_store = Box::new(RamContentStore::default());
        let mut manifest = Manifest::default();

        let asset_id = {
            let id = ResourceId::new(test_asset::TestAsset::TYPE, 1);
            let checksum = content_store.store(content).unwrap();
            manifest.insert(id, checksum, content.len());
            id
        };

        let reg = AssetRegistryOptions::new()
            .add_device_cas(content_store, manifest)
            .add_loader::<test_asset::TestAsset>()
            .create();

        (asset_id, reg)
    }

    fn setup_dependency_test() -> (ResourceId, ResourceId, Arc<AssetRegistry>) {
        let mut content_store = Box::new(RamContentStore::default());
        let mut manifest = Manifest::default();

        let binary_parent_assetfile = [
            97, 115, 102, 116, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            86, 63, 214, 53, 86, 63, 214, 53, 1, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 112,
            97, 114, 101, 110, 116,
        ];
        let binary_child_assetfile = [
            97, 115, 102, 116, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 63, 214, 53, 1, 0, 0, 0, 0, 0, 0,
            0, 5, 0, 0, 0, 0, 0, 0, 0, 99, 104, 105, 108, 100,
        ];

        let child_id = ResourceId::new(test_asset::TestAsset::TYPE, 1);

        let parent_id = {
            manifest.insert(
                child_id,
                content_store.store(&binary_child_assetfile).unwrap(),
                binary_child_assetfile.len(),
            );
            let checksum = content_store.store(&binary_parent_assetfile).unwrap();
            let id = ResourceId::new(test_asset::TestAsset::TYPE, 2);
            manifest.insert(id, checksum, binary_parent_assetfile.len());
            id
        };

        let reg = AssetRegistryOptions::new()
            .add_device_cas(content_store, manifest)
            .add_loader::<test_asset::TestAsset>()
            .create();

        (parent_id, child_id, reg)
    }

    const BINARY_ASSETFILE: [u8; 39] = [
        97, 115, 102, 116, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 63, 214, 53, 1, 0, 0, 0, 0, 0, 0, 0,
        5, 0, 0, 0, 0, 0, 0, 0, 99, 104, 105, 108, 100,
    ];

    const BINARY_RAWFILE: [u8; 5] = [99, 104, 105, 108, 100];

    #[test]
    fn load_assetfile() {
        let (asset_id, reg) = setup_singular_asset_test(&BINARY_ASSETFILE);

        let internal_id;
        {
            let a = reg.load_untyped(asset_id);
            internal_id = a.id();

            let mut test_timeout = Duration::from_millis(500);
            while test_timeout > Duration::ZERO && !a.is_loaded(&reg) {
                let sleep_time = Duration::from_millis(10);
                thread::sleep(sleep_time);
                test_timeout -= sleep_time;
                reg.update();
            }

            assert!(a.is_loaded(&reg));
            assert!(!a.is_err(&reg));
            assert!(reg.is_loaded(internal_id));
            {
                let b = a.clone();
                reg.update();
                assert_eq!(a, b);

                assert!(b.is_loaded(&reg));
                assert!(!b.is_err(&reg));
                assert!(reg.is_loaded(internal_id));
            }
        }
        reg.update();
        assert!(!reg.is_loaded(internal_id));
    }

    #[test]
    fn load_rawfile() {
        let (asset_id, reg) = setup_singular_asset_test(&BINARY_RAWFILE);

        let internal_id;
        {
            let a = reg.load_untyped(asset_id);
            internal_id = a.id();

            let mut test_timeout = Duration::from_millis(500);
            while test_timeout > Duration::ZERO && !a.is_loaded(&reg) {
                let sleep_time = Duration::from_millis(10);
                thread::sleep(sleep_time);
                test_timeout -= sleep_time;
                reg.update();
            }

            assert!(a.is_loaded(&reg));
            assert!(!a.is_err(&reg));
            assert!(reg.is_loaded(internal_id));
            {
                let b = a.clone();
                reg.update();
                assert_eq!(a, b);

                assert!(b.is_loaded(&reg));
                assert!(!b.is_err(&reg));
                assert!(reg.is_loaded(internal_id));
            }
        }
        reg.update();
        assert!(!reg.is_loaded(internal_id));
    }

    #[test]
    fn load_error() {
        let (_, reg) = setup_singular_asset_test(&BINARY_ASSETFILE);

        let internal_id;
        {
            let a = reg.load_untyped(ResourceId::new(test_asset::TestAsset::TYPE, 7));
            internal_id = a.id();

            let mut test_timeout = Duration::from_millis(500);
            while test_timeout > Duration::ZERO && !a.is_err(&reg) {
                let sleep_time = Duration::from_millis(10);
                thread::sleep(sleep_time);
                test_timeout -= sleep_time;
                reg.update();
            }

            assert!(!a.is_loaded(&reg));
            assert!(a.is_err(&reg));
            assert!(!reg.is_loaded(internal_id));
        }
        reg.update();
        assert!(!reg.is_loaded(internal_id));
    }

    #[test]
    fn load_error_sync() {
        let (_, reg) = setup_singular_asset_test(&BINARY_ASSETFILE);

        let internal_id;
        {
            let a = reg.load_untyped_sync(ResourceId::new(test_asset::TestAsset::TYPE, 7));
            internal_id = a.id();

            assert!(!a.is_loaded(&reg));
            assert!(a.is_err(&reg));
            assert!(!reg.is_loaded(internal_id));
        }
        reg.update();
        assert!(!reg.is_loaded(internal_id));
    }

    #[test]
    fn load_dependency() {
        let (parent_id, child_id, reg) = setup_dependency_test();

        let parent = reg.load_untyped_sync(parent_id);
        assert!(parent.is_loaded(&reg));

        let child = reg.load_untyped(child_id);
        assert!(
            child.is_loaded(&reg),
            "The dependency should immediately be considered as loaded"
        );

        std::mem::drop(parent);
        reg.update();

        assert!(reg.get_untyped(parent_id).is_none());

        assert!(
            child.is_loaded(&reg),
            "The dependency should be kept alive because of the handle"
        );

        std::mem::drop(child);
        reg.update();
        assert!(reg.get_untyped(child_id).is_none());
    }
}
