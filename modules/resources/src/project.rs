use crate::metadata::Metadata;
use crate::metadata::ResourceHash;
use crate::types::ResourceId;
use crate::types::ResourceType;
use crate::ResourcePathRef;

use crate::ResourcePath;

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Seek;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const METADATA_EXT: &str = "meta";
const RESOURCE_EXT: &str = "blob";

/// A project exists always within a given directory and this file
/// will be created directly in that directory.
const PROJECT_INDEX_FILENAME: &str = ".project.index";

#[derive(Serialize, Deserialize, Default)]
struct ResourceDb {
    remote_resources: Vec<ResourceId>,
    local_resources: Vec<ResourceId>,
}

/// A file-backed state of the project
///
/// This structure captures the state of the project. This includes `remote resources`
/// pulled from `source-control` as well as `local resources` added/removed/edited locally.
///
/// It provides a resource-oriented interface to source-control.
pub struct Project {
    file: std::fs::File,
    db: ResourceDb,
    projectroot_path: PathBuf,
}

#[derive(Debug)]
/// Error returned by the project.
pub enum Error {
    /// Project index parsing error.
    ParseError,
    /// Not found.
    NotFound,
    /// Specified path is invalid.
    InvalidPath,
    /// IO error on the project index file.
    IOError(std::io::Error), // todo(kstasik): have clearer Open/Read/Write errors that will be easier to handle layer above
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::ParseError => write!(f, "Error Parsing Content"),
            Error::NotFound => write!(f, "Resource Not Found"),
            Error::InvalidPath => write!(f, "Path Not Found"),
            Error::IOError(ref err) => err.fmt(f),
        }
    }
}

impl Project {
    /// Returns the default location of the index file in a given directory.
    ///
    /// This method replaces the filename in `work_dir` (if one exists) with
    /// the file name of the project index.
    pub fn root_to_index_path(projectroot_path: &Path) -> PathBuf {
        let mut path = projectroot_path.to_owned();
        if path.is_dir() {
            path.push(PROJECT_INDEX_FILENAME);
        } else {
            path.set_file_name(PROJECT_INDEX_FILENAME);
        }
        path
    }

    /// Returns the path to project's index file.
    pub fn indexfile_path(&self) -> PathBuf {
        Self::root_to_index_path(&self.projectroot_path)
    }

    /// Creates a new project index file turining the containing directory into a project.
    pub fn create_new(projectroot_path: &Path) -> Result<Self, Error> {
        let index_path = Self::root_to_index_path(projectroot_path);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&index_path)
            .map_err(Error::IOError)?;

        let db = ResourceDb::default();
        serde_json::to_writer(&file, &db).map_err(|_e| Error::ParseError)?;

        let projectroot_path = index_path.parent().unwrap().to_owned();
        Ok(Self {
            file,
            db,
            projectroot_path,
        })
    }

    /// Opens the project index specified
    pub fn open(projectroot_path: &Path) -> Result<Self, Error> {
        let index_path = Self::root_to_index_path(projectroot_path);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .append(false)
            .open(&index_path)
            .map_err(|_e| Error::NotFound)?;

        let db = serde_json::from_reader(&file).map_err(|_e| Error::ParseError)?;

        let projectroot_path = index_path.parent().unwrap().to_owned();
        Ok(Self {
            file,
            db,
            projectroot_path,
        })
    }

    /// Deletes the project by deleting the index file.
    pub fn delete(self) {
        let index_path = self.indexfile_path();
        let _res = fs::remove_file(index_path);
    }

    /// Returns the list resources.
    ///
    /// This method flattens the `remote` and `local` resources into one list.
    pub fn resource_list(&self) -> Vec<ResourceId> {
        let all_resources = [&self.db.remote_resources, &self.db.local_resources];
        let references = all_resources.iter().flat_map(|v| v.iter());
        references.cloned().collect()
    }

    /// Finds resource by its name and returns its `ResourceId`.
    pub fn find_resource(&self, name: &ResourcePathRef) -> Result<ResourceId, Error> {
        let all_resources = [&self.db.remote_resources, &self.db.local_resources];
        let mut references = all_resources.iter().flat_map(|v| v.iter());

        // this below would be better expressed as try_map (still experimental).
        let res = references.find_map(|id| match self.read_meta(*id) {
            Ok(meta) => {
                if meta.name == name {
                    Some(Ok(*id))
                } else {
                    None
                }
            }
            Err(e) => Some(Err(e)),
        });

        match res {
            None => Err(Error::NotFound),
            Some(e) => e,
        }
    }

    /// Checks if a resource exists.
    pub fn exists(&self, name: &ResourcePathRef) -> bool {
        self.find_resource(name).is_ok()
    }

    /// Reads the resource content file.
    pub fn read_resource(&self, id: ResourceId) -> Result<Vec<u8>, Error> {
        let resource_path = self.resource_path(id);

        let data = fs::read(resource_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::NotFound,
            _ => Error::IOError(e),
        })?;
        Ok(data)
    }

    /// Changes the content of the resource to `new_content` and updates the corresponding .meta file.
    pub fn save_resource(&mut self, id: ResourceId, new_content: &[u8]) -> Result<(), Error> {
        let resource_path = self.resource_path(id);
        let metadata_path = self.metadata_path(id);

        let mut meta_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(metadata_path)
            .map_err(Error::IOError)?;
        let mut resource_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(resource_path)
            .map_err(Error::IOError)?;
        let mut metadata: Metadata =
            serde_json::from_reader(&meta_file).map_err(|_e| Error::ParseError)?;

        resource_file.write_all(new_content).unwrap(); // todo(kstasik): introduce 'corrupted' resource flag.

        metadata.content_checksum = {
            let mut hasher = DefaultHasher::new();
            new_content.hash(&mut hasher);
            hasher.finish() as i128
        };

        // todo(kstasik): parse the resource to extract dependencies.

        meta_file.set_len(0).unwrap();
        meta_file.seek(std::io::SeekFrom::Start(0)).unwrap();
        serde_json::to_writer_pretty(&meta_file, &metadata).unwrap(); // todo(kstasik): same as above.
        Ok(())
    }

    /// Creates an empty resource file of a given type with an associated `.meta`.
    ///
    /// The created `.meta` file contains a checksum of the resource content.
    /// `TODO`: the checksum of content needs to be updated when file is modified.
    ///
    /// Both resource file and its corresponding `.meta` file are `staged`.
    /// Use [`Self::commit()`] to push changes to remote.
    pub fn create_resource_with_deps(
        &mut self,
        name: ResourcePath,
        kind: ResourceType,
        dependencies: &[ResourceId],
        content: &[u8],
    ) -> Result<ResourceId, Error> {
        let id = ResourceId::generate_new(kind);

        let meta_path = self.metadata_path(id);
        let mut resource_path = meta_path.clone();
        resource_path.set_extension(RESOURCE_EXT);

        let mut resource_file = File::create(&resource_path).map_err(Error::IOError)?;

        resource_file.write_all(content).map_err(|e| {
            fs::remove_file(&resource_path).unwrap();
            Error::IOError(e)
        })?;

        let meta_file = File::create(&meta_path).map_err(|e| {
            fs::remove_file(&resource_path).unwrap();
            Error::IOError(e)
        })?;

        let content_checksum = {
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            hasher.finish() as i128
        };

        let metadata = Metadata::new_with_dependencies(name, content_checksum, dependencies);
        serde_json::to_writer_pretty(meta_file, &metadata).unwrap();

        self.db.local_resources.push(id);
        Ok(id)
    }

    /// Creates an empty resource file of a given type with an associated `.meta`.
    ///
    /// For more information see [`Self::create_resource_with_deps()`].
    pub fn create_resource(
        &mut self,
        name: ResourcePath,
        kind: ResourceType,
        content: &[u8],
    ) -> Result<ResourceId, Error> {
        self.create_resource_with_deps(name, kind, &[], content)
    }

    /// Gathers information about a given resource.
    ///
    /// This method opens `.meta` file of the requested resource and all its dependent resources.
    ///
    /// `TODO`: This implementation does a lot of IO work. It will become inefficient quickly.
    /// Caching and related cache invalidation when pulling from source-control and/or when modifying assets locally
    /// will be key here.
    pub fn collect_resource_info(
        &self,
        id: ResourceId,
    ) -> Result<(ResourceHash, Vec<ResourceId>), Error> {
        let mut dependencies = Vec::<ResourceId>::new();

        let mut queue = Vec::<ResourceId>::new();

        let gather_dependencies =
            |queue: &mut Vec<ResourceId>, dependencies: &mut Vec<ResourceId>, meta: &Metadata| {
                for dep in &meta.build_deps {
                    if !dependencies.contains(dep) {
                        dependencies.push(*dep);
                        queue.push(*dep);
                    }
                }
            };

        let meta = self.read_meta(id)?;
        gather_dependencies(&mut queue, &mut dependencies, &meta);
        let resource_hash = meta.resource_hash();

        while let Some(id) = queue.pop() {
            let meta = self.read_meta(id)?;
            gather_dependencies(&mut queue, &mut dependencies, &meta);
        }

        Ok((resource_hash, dependencies))
    }

    fn metadata_path(&self, id: ResourceId) -> PathBuf {
        let mut path = self.projectroot_path.clone();
        path.push(format!("{:x}", id));
        path.set_extension(METADATA_EXT);
        path
    }

    fn resource_path(&self, id: ResourceId) -> PathBuf {
        let mut path = self.projectroot_path.clone();
        path.push(format!("{:x}", id));
        path.set_extension(RESOURCE_EXT);
        path
    }

    /// Moves a `remote` resources to the list of `local` resources.
    pub fn checkout(&mut self, id: ResourceId) -> Result<(), Error> {
        if let Some(_resource) = self.db.local_resources.iter().find(|&res| *res == id) {
            return Ok(()); // already checked out
        }

        if let Some(index) = self.db.remote_resources.iter().position(|res| *res == id) {
            let resource = self.db.remote_resources.remove(index);
            self.db.local_resources.push(resource);
            return Ok(());
        }

        Err(Error::NotFound)
    }

    fn read_meta(&self, id: ResourceId) -> Result<Metadata, Error> {
        let path = self.metadata_path(id);

        let file = File::open(path).map_err(Error::IOError)?;

        let result = serde_json::from_reader(file).map_err(|_e| Error::ParseError)?;
        Ok(result)
    }

    fn update_meta<F>(&self, id: ResourceId, mut func: F)
    where
        F: FnMut(&mut Metadata),
    {
        let path = self.metadata_path(id);

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap(); // todo(kstasik): return a result and propagate an error

        let mut meta = serde_json::from_reader(&file).unwrap();

        func(&mut meta);

        file.set_len(0).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        serde_json::to_writer_pretty(&file, &meta).unwrap();
    }

    /// Change the name of the resource.
    ///
    /// Changing the name of the resource if `free`. It does not change its `ResourceId`
    /// nor it invalidates any build using that asset.
    pub fn rename_resource(
        &mut self,
        id: ResourceId,
        new_name: &ResourcePathRef,
    ) -> Result<ResourcePath, Error> {
        self.checkout(id)?;

        let mut old_name = ResourcePath::new();
        self.update_meta(id, |data| {
            old_name = data.rename(new_name.to_owned());
        });
        Ok(old_name)
    }

    /// Moves `local` resources to `remote` resource list.
    pub fn commit(&mut self) -> Result<(), Error> {
        self.db
            .remote_resources
            .append(&mut self.db.local_resources);
        self.flush()
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.file.set_len(0).unwrap();
        self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
        serde_json::to_writer_pretty(&self.file, &self.db).map_err(|_e| Error::ParseError)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        // todo(kstasik): writing to a file on drop can be problematic
        self.flush().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, path::Path};

    use tempfile::TempDir;

    use crate::{project::Project, types::ResourceType, ResourcePath};

    use super::ResourceDb;

    fn setup_test() -> TempDir {
        let root = tempfile::tempdir().unwrap();

        let projectindex_path = Project::root_to_index_path(root.path());
        let projectindex_file = File::create(projectindex_path).unwrap();

        serde_json::to_writer(projectindex_file, &ResourceDb::default()).unwrap();
        root
    }

    fn create_actor(projectroot_path: &Path) -> Project {
        let index_path = Project::root_to_index_path(projectroot_path);
        let mut project = Project::open(&index_path).unwrap();
        let texture = project
            .create_resource(
                ResourcePath::from("albedo.texture"),
                ResourceType::Texture,
                b"test",
            )
            .unwrap();
        let material = project
            .create_resource_with_deps(
                ResourcePath::from("body.material"),
                ResourceType::Material,
                &[texture],
                b"test",
            )
            .unwrap();
        let geometry = project
            .create_resource_with_deps(
                ResourcePath::from("hero.geometry"),
                ResourceType::Geometry,
                &[material],
                b"test",
            )
            .unwrap();
        let skeleton = project
            .create_resource(
                ResourcePath::from("hero.skeleton"),
                ResourceType::Skeleton,
                b"test",
            )
            .unwrap();
        let _actor = project
            .create_resource_with_deps(
                ResourcePath::from("hero.actor"),
                ResourceType::Actor,
                &[geometry, skeleton],
                b"test",
            )
            .unwrap();

        project
    }

    fn create_sky_material(project: &mut Project) {
        let texture = project
            .create_resource(
                ResourcePath::from("sky.texture"),
                ResourceType::Texture,
                b"test",
            )
            .unwrap();
        let _material = project
            .create_resource_with_deps(
                ResourcePath::from("sky.material"),
                ResourceType::Material,
                &[texture],
                b"test",
            )
            .unwrap();
    }

    /*
    // + data-offline/
    //  - albedo.texture
    //  - body.metarial // texture ref
    //  - hero.geometry // material ref
    //  - hero.actor // geometry ref, skeleton ref
    //  - hero.skeleton // no refs
     */

    #[test]
    fn proj_create_delete() {
        let root = tempfile::tempdir().unwrap();

        let project = Project::create_new(root.path()).expect("faild to create project");
        let same_project = Project::create_new(root.path());
        assert!(same_project.is_err());

        project.delete();

        let _project = Project::create_new(root.path()).expect("faild to re-create project");
        let same_project = Project::create_new(root.path());
        assert!(same_project.is_err());
    }

    #[test]
    fn local_changes() {
        let projroot_path = setup_test();
        let project = create_actor(projroot_path.path());

        assert_eq!(project.db.local_resources.len(), 5);
        assert_eq!(project.db.remote_resources.len(), 0);
    }

    #[test]
    fn commit() {
        let projroot_path = setup_test();
        let mut project = create_actor(projroot_path.path());

        project.commit().unwrap();

        assert_eq!(project.db.local_resources.len(), 0);
        assert_eq!(project.db.remote_resources.len(), 5);
    }

    #[test]
    fn collect_dependencies() {
        let projectroot_path = setup_test();
        let project = create_actor(projectroot_path.path());

        let top_level_resource = project
            .find_resource(&ResourcePath::from("hero.actor"))
            .unwrap();

        let (_, all_deps) = project.collect_resource_info(top_level_resource).unwrap();

        assert_eq!(all_deps.len(), 4);
    }

    #[test]
    fn rename() {
        let rename_assert = |proj: &mut Project, old_name: ResourcePath, new_name: ResourcePath| {
            let skeleton_id = proj.find_resource(&old_name);
            assert!(skeleton_id.is_ok());
            let skeleton_id = skeleton_id.unwrap();

            let prev_name = proj.rename_resource(skeleton_id, &new_name);
            assert!(prev_name.is_ok());
            let prev_name = prev_name.unwrap();
            assert_eq!(&prev_name, &old_name);

            assert!(proj.find_resource(&old_name).is_err());
            assert_eq!(proj.find_resource(&new_name).unwrap(), skeleton_id);
        };

        let projectroot_path = setup_test();
        let mut project = create_actor(projectroot_path.path());
        assert!(project.commit().is_ok());
        create_sky_material(&mut project);

        rename_assert(
            &mut project,
            ResourcePath::from("hero.skeleton"),
            ResourcePath::from("boss.skeleton"),
        );
        rename_assert(
            &mut project,
            ResourcePath::from("sky.material"),
            ResourcePath::from("clouds.material"),
        );
    }
}
