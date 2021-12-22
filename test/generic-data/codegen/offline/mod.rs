#[path = "../offline/debug_cube.rs"]
mod debug_cube;
pub use debug_cube::*;

#[path = "../offline/entity_dc.rs"]
mod entity_dc;
pub use entity_dc::*;

#[path = "../offline/instance_dc.rs"]
mod instance_dc;
pub use instance_dc::*;

#[path = "../offline/light_component.rs"]
mod light_component;
pub use light_component::*;

#[path = "../offline/rotation_component.rs"]
mod rotation_component;
pub use rotation_component::*;

#[path = "../offline/static_mesh_component.rs"]
mod static_mesh_component;
pub use static_mesh_component::*;

#[path = "../offline/test_entity.rs"]
mod test_entity;
pub use test_entity::*;

#[path = "../offline/transform_component.rs"]
mod transform_component;
pub use transform_component::*;

pub fn register_resource_types(
    registry: lgn_data_offline::resource::ResourceRegistryOptions,
) -> lgn_data_offline::resource::ResourceRegistryOptions {
    registry
        .add_type::<DebugCube>()
        .add_type::<EntityDc>()
        .add_type::<InstanceDc>()
        .add_type::<TestEntity>()
}