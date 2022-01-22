//! Offline data structs used in the sample-data test

// crate-specific lint exceptions:
//#![allow()]

use std::{
    any::{Any, TypeId},
    io,
};

use lgn_data_offline::{
    resource::{OfflineResource, ResourceProcessor, ResourceRegistryOptions},
    ResourcePathId,
};
use lgn_data_runtime::{resource, Asset, AssetLoader, Resource};
use lgn_math::prelude::*;
use serde::{Deserialize, Serialize};

pub fn register_resource_types(registry: &mut ResourceRegistryOptions) {
    registry
        .add_type_mut::<Entity>()
        .add_type_mut::<Instance>()
        .add_type_mut::<Mesh>()
        .add_type_mut::<Script>();
}

// ------------------ Entity -----------------------------------

#[resource("offline_entity")]
#[derive(Default, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub children: Vec<ResourcePathId>,
    pub parent: Option<ResourcePathId>,
    pub components: Vec<Box<dyn Component>>,
}

impl Asset for Entity {
    type Loader = EntityProcessor;
}

impl OfflineResource for Entity {
    type Processor = EntityProcessor;
}

#[derive(Default)]
pub struct EntityProcessor {}

impl AssetLoader for EntityProcessor {
    fn load(&mut self, reader: &mut dyn io::Read) -> io::Result<Box<dyn Any + Send + Sync>> {
        let result: Entity = serde_json::from_reader(reader)?;
        Ok(Box::new(result))
    }

    fn load_init(&mut self, _asset: &mut (dyn Any + Send + Sync)) {}
}

impl ResourceProcessor for EntityProcessor {
    fn new_resource(&mut self) -> Box<dyn Any + Send + Sync> {
        Box::new(Entity::default())
    }

    fn extract_build_dependencies(&mut self, resource: &dyn Any) -> Vec<ResourcePathId> {
        let entity = resource.downcast_ref::<Entity>().unwrap();
        let mut deps = entity.children.clone();
        for comp in &entity.components {
            deps.extend(comp.extract_build_deps());
        }
        deps
    }

    fn write_resource(
        &self,
        resource: &dyn Any,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<usize> {
        let resource = resource.downcast_ref::<Entity>().unwrap();
        serde_json::to_writer_pretty(writer, resource).unwrap();
        Ok(1) // no bytes written exposed by serde.
    }

    fn read_resource(
        &mut self,
        reader: &mut dyn std::io::Read,
    ) -> std::io::Result<Box<dyn Any + Send + Sync>> {
        self.load(reader)
    }
}

#[typetag::serde]
pub trait Component: Any + Sync + Send {
    fn extract_build_deps(&self) -> Vec<ResourcePathId>;
}

/// Note: Based on impl of dyn Any
impl dyn Component {
    /// Returns `true` if the boxed type is the same as `T`.
    /// (See [`std::any::Any::is`](https://doc.rust-lang.org/std/any/trait.Any.html#method.is))
    #[inline]
    pub fn is<T: Component>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    /// (See [`std::any::Any::downcast_ref`](https://doc.rust-lang.org/std/any/trait.Any.html#method.downcast_ref))
    #[inline]
    pub fn downcast_ref<T: Component>(&self) -> Option<&T> {
        if self.is::<T>() {
            #[allow(unsafe_code)]
            unsafe {
                Some(&*((self as *const dyn Component).cast::<T>()))
            }
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub apply_to_children: bool,
}

#[typetag::serde]
impl Component for Transform {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
pub struct Visual {
    pub renderable_geometry: Option<ResourcePathId>,
    pub shadow_receiver: bool,
    pub shadow_caster_sun: bool,
    pub shadow_caster_local: bool,
    pub gi_contribution: GIContribution,
}

#[typetag::serde]
impl Component for Visual {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        if let Some(rg) = &self.renderable_geometry {
            vec![rg.clone()]
        } else {
            vec![]
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum GIContribution {
    Default,
    Blocker,
    Exclude,
}

#[derive(Serialize, Deserialize)]
pub struct GlobalIllumination {}

#[typetag::serde]
impl Component for GlobalIllumination {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
pub struct NavMesh {
    pub voxelisation_config: VoxelisationConfig,
    pub layer_config: Vec<NavMeshLayerConfig>,
}

#[typetag::serde]
impl Component for NavMesh {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
pub struct VoxelisationConfig {}

#[derive(Serialize, Deserialize)]
pub struct NavMeshLayerConfig {}

#[derive(Serialize, Deserialize)]
pub struct View {
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub projection_type: ProjectionType,
}

#[typetag::serde]
impl Component for View {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
pub enum ProjectionType {
    Orthogonal,
    Perspective,
}

#[derive(Serialize, Deserialize)]
pub struct Light {}

#[typetag::serde]
impl Component for Light {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
pub struct Physics {
    pub dynamic: bool,
    pub collision_geometry: Option<ResourcePathId>,
}

#[typetag::serde]
impl Component for Physics {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        if let Some(cg) = &self.collision_geometry {
            vec![cg.clone()]
        } else {
            vec![]
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StaticMesh {
    pub mesh_id: usize,
}

#[typetag::serde]
impl Component for StaticMesh {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        vec![]
    }
}

// ------------------ Script -----------------------------------

#[resource("offline_script")]
#[derive(Serialize, Deserialize)]
pub struct Script {
    pub script: String,
}

impl Asset for Script {
    type Loader = ScriptProcessor;
}

impl OfflineResource for Script {
    type Processor = ScriptProcessor;
}

#[derive(Default)]
pub struct ScriptProcessor {}

impl AssetLoader for ScriptProcessor {
    fn load(&mut self, reader: &mut dyn io::Read) -> io::Result<Box<dyn Any + Send + Sync>> {
        let mut script = String::new();
        reader.read_to_string(&mut script)?;
        Ok(Box::new(Script { script }))
    }

    fn load_init(&mut self, _asset: &mut (dyn Any + Send + Sync)) {}
}

impl ResourceProcessor for ScriptProcessor {
    fn new_resource(&mut self) -> Box<dyn Any + Send + Sync> {
        Box::new(Script {
            script: String::new(),
        })
    }

    fn extract_build_dependencies(&mut self, _resource: &dyn Any) -> Vec<ResourcePathId> {
        vec![]
    }

    fn write_resource(
        &self,
        resource: &dyn Any,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<usize> {
        let script = resource.downcast_ref::<Script>().unwrap();
        writer.write_all(script.script.as_bytes())?;
        Ok(1) // no bytes written exposed by serde.
    }

    fn read_resource(
        &mut self,
        reader: &mut dyn std::io::Read,
    ) -> std::io::Result<Box<dyn Any + Send + Sync>> {
        self.load(reader)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ScriptComponent {
    pub input_values: Vec<String>,
    pub entry_fn: String,
    pub script: Option<ResourcePathId>,
}

#[typetag::serde]
impl Component for ScriptComponent {
    fn extract_build_deps(&self) -> Vec<ResourcePathId> {
        if let Some(s) = &self.script {
            vec![s.clone()]
        } else {
            vec![]
        }
    }
}

// ------------------ Instance  -----------------------------------

#[resource("offline_instance")]
#[derive(Serialize, Deserialize)]
pub struct Instance {
    pub original: Option<ResourcePathId>,
}

impl Asset for Instance {
    type Loader = InstanceProcessor;
}

impl OfflineResource for Instance {
    type Processor = InstanceProcessor;
}

#[derive(Default)]
pub struct InstanceProcessor {}

impl AssetLoader for InstanceProcessor {
    fn load(&mut self, reader: &mut dyn io::Read) -> io::Result<Box<dyn Any + Send + Sync>> {
        let result: Instance = serde_json::from_reader(reader)?;
        Ok(Box::new(result))
    }

    fn load_init(&mut self, _asset: &mut (dyn Any + Send + Sync)) {}
}

impl ResourceProcessor for InstanceProcessor {
    fn new_resource(&mut self) -> Box<dyn Any + Send + Sync> {
        Box::new(Instance { original: None })
    }

    fn extract_build_dependencies(&mut self, resource: &dyn Any) -> Vec<ResourcePathId> {
        let instance = resource.downcast_ref::<Instance>().unwrap();
        instance.original.iter().cloned().collect()
    }

    fn write_resource(
        &self,
        resource: &dyn Any,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<usize> {
        let resource = resource.downcast_ref::<Instance>().unwrap();
        serde_json::to_writer_pretty(writer, resource).unwrap();
        Ok(1) // no bytes written exposed by serde.
    }

    fn read_resource(
        &mut self,
        reader: &mut dyn std::io::Read,
    ) -> std::io::Result<Box<dyn Any + Send + Sync>> {
        self.load(reader)
    }
}

// ------------------ Mesh -----------------------------------

#[resource("offline_mesh")]
#[derive(Serialize, Deserialize)]
pub struct Mesh {
    pub sub_meshes: Vec<SubMesh>,
}

impl Asset for Mesh {
    type Loader = MeshProcessor;
}

impl OfflineResource for Mesh {
    type Processor = MeshProcessor;
}

#[derive(Default)]
pub struct MeshProcessor {}

impl AssetLoader for MeshProcessor {
    fn load(&mut self, reader: &mut dyn io::Read) -> io::Result<Box<dyn Any + Send + Sync>> {
        let result: Mesh = serde_json::from_reader(reader)?;
        Ok(Box::new(result))
    }

    fn load_init(&mut self, _asset: &mut (dyn Any + Send + Sync)) {}
}

impl ResourceProcessor for MeshProcessor {
    fn new_resource(&mut self) -> Box<dyn Any + Send + Sync> {
        Box::new(Mesh {
            sub_meshes: Vec::default(),
        })
    }

    fn extract_build_dependencies(&mut self, resource: &dyn Any) -> Vec<ResourcePathId> {
        let mesh = resource.downcast_ref::<Mesh>().unwrap();
        let mut material_refs: Vec<ResourcePathId> = mesh
            .sub_meshes
            .iter()
            .filter_map(|sub_mesh| sub_mesh.material.as_ref())
            .cloned()
            .collect();
        material_refs.sort();
        material_refs.dedup();
        material_refs
    }

    fn write_resource(
        &self,
        resource: &dyn Any,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<usize> {
        let resource = resource.downcast_ref::<Mesh>().unwrap();
        serde_json::to_writer_pretty(writer, resource).unwrap();
        Ok(1) // no bytes written exposed by serde.
    }

    fn read_resource(
        &mut self,
        reader: &mut dyn std::io::Read,
    ) -> std::io::Result<Box<dyn Any + Send + Sync>> {
        self.load(reader)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SubMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u16>,
    pub material: Option<ResourcePathId>,
}