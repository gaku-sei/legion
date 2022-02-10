use std::collections::HashSet;
use std::sync::Arc;

use lgn_data_offline::resource::{Project, ResourceHandles, ResourceRegistry};
use lgn_data_runtime::{AssetRegistry, ResourceTypeAndId};
use lgn_tracing::error;
use tokio::sync::MutexGuard;

use crate::{BuildManager, DataManager, Error};

/// Describe a Lock on the Database (Project/ResourceRegistry/LoadedResources)
pub struct LockContext<'a> {
    /// Lock on the `Project`
    pub project: MutexGuard<'a, Project>,
    /// Lock on the `ResourceRegistry`
    pub resource_registry: MutexGuard<'a, ResourceRegistry>,
    /// Lock on the LoadedResources
    pub loaded_resource_handles: MutexGuard<'a, ResourceHandles>,
    /// Reference to the Asset Registry
    pub asset_registry: Arc<AssetRegistry>,
    /// Reference to build manager.
    pub build: MutexGuard<'a, BuildManager>,
    // List of Resource changed during the lock (that need saving)
    pub(crate) changed_resources: HashSet<ResourceTypeAndId>,
}

impl<'a> LockContext<'a> {
    /// Create a new Lock on the `DataManager`
    pub async fn new(data_manager: &'a DataManager) -> LockContext<'a> {
        Self {
            project: data_manager.project.lock().await,
            resource_registry: data_manager.resource_registry.lock().await,
            asset_registry: data_manager.asset_registry.clone(),
            build: data_manager.build_manager.lock().await,
            loaded_resource_handles: data_manager.loaded_resource_handles.lock().await,
            changed_resources: HashSet::new(),
        }
    }

    pub(crate) async fn save_changed_resources(&mut self) -> Result<(), Error> {
        for resource_id in &self.changed_resources {
            if let Some(handle) = self.loaded_resource_handles.get(*resource_id) {
                self.project
                    .save_resource(*resource_id, &handle, &mut self.resource_registry)
                    .await
                    .map_err(|err| Error::Project(*resource_id, err))?;
            }
        }

        for resource_id in &self.changed_resources {
            match self
                .build
                .build_all_derived(*resource_id, &self.project)
                .await
            {
                Ok((runtime_path_id, _built_resources)) => {
                    self.asset_registry.reload(runtime_path_id.resource_id());
                }
                Err(e) => {
                    error!("Error building resource derivations {:?}", e);
                }
            }
        }

        self.resource_registry.collect_garbage();
        Ok(())
    }
}
