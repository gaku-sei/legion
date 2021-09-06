use std::{marker::PhantomData, sync::mpsc};

use super::{Resource, ResourceRegistry};

pub(crate) type ResourceHandleId = u32;

pub(crate) enum RefOp {
    AddRef(ResourceHandleId),
    RemoveRef(ResourceHandleId),
}

/// Type-less version of [`ResourceHandle`].
#[derive(Debug)]
pub struct ResourceHandleUntyped {
    pub(crate) id: ResourceHandleId,
    refcount_tx: mpsc::Sender<RefOp>,
}

impl AsRef<Self> for ResourceHandleUntyped {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Drop for ResourceHandleUntyped {
    fn drop(&mut self) {
        self.refcount_tx.send(RefOp::RemoveRef(self.id)).unwrap();
    }
}

impl Clone for ResourceHandleUntyped {
    fn clone(&self) -> Self {
        self.refcount_tx.send(RefOp::AddRef(self.id)).unwrap();
        Self {
            id: self.id,
            refcount_tx: self.refcount_tx.clone(),
        }
    }
}

impl PartialEq for ResourceHandleUntyped {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl ResourceHandleUntyped {
    pub(crate) fn create(id: ResourceHandleId, refcount_tx: mpsc::Sender<RefOp>) -> Self {
        Self { id, refcount_tx }
    }

    /// Retrieve a reference to resource of type `T` from [`ResourceRegistry`].
    pub fn get<'a, T: Resource>(&'_ self, registry: &'a ResourceRegistry) -> Option<&'a T> {
        let resource = registry.get(self)?;
        resource.as_any().downcast_ref::<T>()
    }

    /// Retrieve a mutable reference to resource of type `T` from [`ResourceRegistry`].
    pub fn get_mut<'a, T: Resource>(
        &'_ self,
        registry: &'a mut ResourceRegistry,
    ) -> Option<&'a mut T> {
        let resource = registry.get_mut(self)?;
        resource.as_any_mut().downcast_mut::<T>()
    }

    /// Converts the untyped handle into a typed handle.
    pub fn typed<T: Resource>(self) -> ResourceHandle<T> {
        let v = ResourceHandle::<T>::create(self.id, self.refcount_tx.clone());
        // the intent here is to not decrement the refcount as the newly returned `v` will take care of it
        // when it goes out of scope. mem::forget stops the destructor of self from running.
        #[allow(clippy::mem_forget)]
        std::mem::forget(self);
        v
    }
}

/// Typed handle to [`Resource`] of type `T`.
pub struct ResourceHandle<T: Resource> {
    internal: ResourceHandleUntyped,
    _pd: PhantomData<fn() -> T>,
}

impl<T: Resource> AsRef<ResourceHandleUntyped> for ResourceHandle<T> {
    fn as_ref(&self) -> &ResourceHandleUntyped {
        &self.internal
    }
}

impl<T: Resource> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        let cloned = self.internal.clone();
        Self {
            internal: cloned,
            _pd: PhantomData {},
        }
    }
}

impl<T: Resource> PartialEq for ResourceHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.internal.id == other.internal.id
    }
}

impl<T: Resource> ResourceHandle<T> {
    pub(crate) fn create(id: ResourceHandleId, refcount_tx: mpsc::Sender<RefOp>) -> Self {
        Self {
            internal: ResourceHandleUntyped::create(id, refcount_tx),
            _pd: PhantomData {},
        }
    }

    /// Retrieve a reference to resource of type `T` from [`ResourceRegistry`].
    pub fn get<'a>(&'_ self, registry: &'a ResourceRegistry) -> Option<&'a T> {
        let resource = registry.get(&self.internal)?;
        resource.as_any().downcast_ref::<T>()
    }

    /// Retrieve a mutable reference to resource of type `T` from [`ResourceRegistry`].
    pub fn get_mut<'a>(&'_ self, registry: &'a mut ResourceRegistry) -> Option<&'a mut T> {
        let resource = registry.get_mut(&self.internal)?;
        resource.as_any_mut().downcast_mut::<T>()
    }
}
