use legion_data_offline::{resource::ResourceProcessor, ResourcePathId};

use legion_data_runtime::{Resource, ResourceType};
use serde::{Deserialize, Serialize};

pub const TYPE_ID: ResourceType = ResourceType::new(b"multitext_resource");

#[derive(Resource, Serialize, Deserialize)]
pub struct MultiTextResource {
    pub text_list: Vec<String>,
}

pub struct MultiTextResourceProc {}

impl ResourceProcessor for MultiTextResourceProc {
    fn new_resource(&mut self) -> Box<dyn Resource> {
        Box::new(MultiTextResource { text_list: vec![] })
    }

    fn extract_build_dependencies(&mut self, _resource: &dyn Resource) -> Vec<ResourcePathId> {
        vec![]
    }

    fn write_resource(
        &mut self,
        resource: &dyn Resource,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<usize> {
        let resource = resource.downcast_ref::<MultiTextResource>().unwrap();
        serde_json::to_writer(writer, resource).unwrap();
        Ok(1) // no bytes written exposed by serde.
    }

    fn read_resource(
        &mut self,
        reader: &mut dyn std::io::Read,
    ) -> std::io::Result<Box<dyn Resource>> {
        let resource: MultiTextResource = serde_json::from_reader(reader).unwrap();
        let boxed = Box::new(resource);
        Ok(boxed)
    }
}
