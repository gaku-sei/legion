//! Transaction Operation to Clone a Resource

use async_trait::async_trait;
#[allow(unused_imports)]
use lgn_data_model::{json_utils::set_property_from_json_string, ReflectionError};
#[allow(unused_imports)]
use lgn_data_runtime::{AssetRegistryReader, ResourceTypeAndId};

use crate::{Error, LockContext, TransactionOperation};

/// Clone a Resource Operation
#[allow(dead_code)]
#[derive(Debug)]
pub struct CloneResourceOperation {
    source_resource_id: ResourceTypeAndId,
    clone_resource_id: ResourceTypeAndId,
    target_parent_id: Option<ResourceTypeAndId>,
}

impl CloneResourceOperation {
    /// Create a new Clone a Resource Operation
    pub fn new(
        source_resource_id: ResourceTypeAndId,
        clone_resource_id: ResourceTypeAndId,
        target_parent_id: Option<ResourceTypeAndId>,
    ) -> Box<Self> {
        Box::new(Self {
            source_resource_id,
            clone_resource_id,
            target_parent_id,
        })
    }
}

#[async_trait]
impl TransactionOperation for CloneResourceOperation {
    async fn apply_operation(&mut self, ctx: &mut LockContext<'_>) -> Result<(), Error> {
        let source_handle = ctx
            .asset_registry
            .load_async_untyped(self.source_resource_id)
            .await?;

        let mut buffer = Vec::<u8>::new();
        ctx.asset_registry
            .serialize_resource(source_handle, &mut buffer)
            .map_err(|err| Error::InvalidResourceSerialization(self.source_resource_id, err))?;

        let reader = Box::pin(std::io::Cursor::new(buffer)) as AssetRegistryReader;

        let clone_handle = ctx
            .asset_registry
            .deserialize_resource(self.clone_resource_id, reader)
            .await
            .map_err(|err| Error::InvalidResourceDeserialization(self.clone_resource_id, err))?;

        // Extract the raw name and check if it's a relative name (with the /!(PARENT_GUID)/
        let mut source_raw_name = ctx
            .project
            .raw_resource_name(self.source_resource_id.id)
            .map_err(|err| Error::Project(self.source_resource_id, err))?;
        source_raw_name.replace_parent_info(self.target_parent_id, None);

        source_raw_name = ctx.project.get_incremental_name(&source_raw_name).await;

        if let Some(entity_name) = source_raw_name.to_string().rsplit('/').next() {
            if let Some(mut resource) = ctx.asset_registry.edit_untyped(&clone_handle) {
                // Try to set the name component field
                if let Err(err) = set_property_from_json_string(
                    resource.as_reflect_mut(),
                    "components[Name].name",
                    &serde_json::json!(entity_name).to_string(),
                ) {
                    match err {
                        ReflectionError::FieldNotFoundOnStruct(_, _)
                        | ReflectionError::ArrayKeyNotFound(_, _) => {} // ignore missing name components
                        _ => return Err(Error::Reflection(self.clone_resource_id, err)),
                    }
                }
                ctx.asset_registry.commit_untyped(resource);
            }
        }

        ctx.project
            .add_resource(source_raw_name, &clone_handle, &ctx.asset_registry)
            .await
            .map_err(|err| Error::Project(self.clone_resource_id, err))?;

        Ok(())
    }

    async fn rollback_operation(&self, ctx: &mut LockContext<'_>) -> Result<(), Error> {
        ctx.project
            .delete_resource(self.clone_resource_id.id)
            .await
            .map_err(|err| Error::Project(self.clone_resource_id, err))?;
        Ok(())
    }
}
