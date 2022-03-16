use std::sync::Arc;

use crate::asset_entities::AssetToEntityMap;
use lgn_core::Name;
use lgn_data_runtime::{AssetRegistry, HandleUntyped, Resource, ResourceTypeAndId};
use lgn_ecs::prelude::*;
use lgn_graphics_renderer::components::{
    LightComponent, LightType, MaterialComponent, Mesh, ModelComponent, TextureComponent,
    TextureData, VisualComponent,
};
use lgn_math::Vec3;

use lgn_tracing::info;
use lgn_transform::prelude::*;
use sample_data::runtime as runtime_data;

pub(crate) fn load_ecs_asset<T>(
    handle: &HandleUntyped,
    registry: &Res<'_, Arc<AssetRegistry>>,
    commands: &mut Commands<'_, '_>,
    asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
    existing_children: Option<&Children>,
) -> bool
where
    T: AssetToECS + Resource + 'static,
{
    let asset_id = &handle.id();
    if asset_id.kind == T::TYPE {
        if let Some(asset) = handle.get::<T>(registry) {
            let entity = T::create_in_ecs(
                commands,
                &asset,
                asset_id,
                registry,
                asset_to_entity_map,
                existing_children,
            );

            if let Some(entity_id) = entity {
                if let Some(old_entity) = asset_to_entity_map.insert(*asset_id, entity_id) {
                    if entity_id.to_bits() != old_entity.to_bits() {
                        commands.entity(old_entity).despawn();
                    }
                }
            }
        }
        true
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) trait AssetToECS {
    fn create_in_ecs(
        _commands: &mut Commands<'_, '_>,
        _asset: &Self,
        _asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        _asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        _existing_children: Option<&Children>,
    ) -> Option<Entity> {
        None
    }
}

impl AssetToECS for runtime_data::Entity {
    fn create_in_ecs(
        commands: &mut Commands<'_, '_>,
        runtime_entity: &Self,
        asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        existing_children: Option<&Children>,
    ) -> Option<Entity> {
        let mut entity = if let Some(entity) = asset_to_entity_map.get(*asset_id) {
            // Look at the existing Ecs Entity children and unspawn
            // the children not present in the data anymore
            if let Some(existing_children) = existing_children {
                for previous_child in existing_children.iter() {
                    if let Some(resource_id) = asset_to_entity_map.get_resource_id(*previous_child)
                    {
                        if runtime_entity
                            .children
                            .iter()
                            .find(|child_ref| child_ref.id() == resource_id)
                            == None
                        {
                            commands.entity(*previous_child).despawn();
                            asset_to_entity_map.remove(*previous_child);
                        }
                    }
                }
            }
            commands.entity(entity)
        } else {
            commands.spawn()
        };

        let mut local_transform: Option<Transform> = None;
        let mut entity_name: Option<String> = None;

        for component in &runtime_entity.components {
            if let Some(transform) = component.downcast_ref::<runtime_data::Transform>() {
                local_transform = Some(Transform {
                    translation: transform.position,
                    rotation: transform.rotation,
                    scale: transform.scale,
                });
            } else if let Some(script) =
                component.downcast_ref::<lgn_scripting::runtime::ScriptComponent>()
            {
                entity.insert(script.clone());
            } else if let Some(name) = component.downcast_ref::<runtime_data::Name>() {
                entity_name = Some(name.name.clone());
            } else if let Some(visual) = component.downcast_ref::<runtime_data::Visual>() {
                entity.insert(VisualComponent::new(
                    &visual.renderable_geometry,
                    visual.color,
                    visual.color_blend,
                ));
            } else if let Some(gi) = component.downcast_ref::<runtime_data::GlobalIllumination>() {
                entity.insert(gi.clone());
            } else if let Some(nav_mesh) = component.downcast_ref::<runtime_data::NavMesh>() {
                entity.insert(nav_mesh.clone());
            } else if let Some(view) = component.downcast_ref::<runtime_data::View>() {
                entity.insert(view.clone());
            } else if let Some(light) = component.downcast_ref::<runtime_data::Light>() {
                entity.insert(LightComponent {
                    light_type: match light.light_type {
                        sample_data::LightType::Omnidirectional => LightType::Omnidirectional,
                        sample_data::LightType::Directional => LightType::Directional,
                        sample_data::LightType::Spotlight => LightType::Spotlight {
                            cone_angle: light.cone_angle,
                        },
                        _ => unreachable!("Unrecognized light type"),
                    },
                    color: Vec3::new(
                        f32::from(light.color.r) / 255.0,
                        f32::from(light.color.g) / 255.0,
                        f32::from(light.color.b) / 255.0,
                    ),
                    radiance: light.radiance,
                    enabled: light.enabled,
                    ..LightComponent::default()
                });
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidBox>()
            {
                entity.insert(physics.clone());
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidCapsule>()
            {
                entity.insert(physics.clone());
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidConvexMesh>()
            {
                entity.insert(physics.clone());
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidHeightField>()
            {
                entity.insert(physics.clone());
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidPlane>()
            {
                entity.insert(physics.clone());
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidSphere>()
            {
                entity.insert(physics.clone());
            } else if let Some(physics) =
                component.downcast_ref::<lgn_physics::runtime::PhysicsRigidTriangleMesh>()
            {
                entity.insert(physics.clone());
            }
        }

        let name = entity_name.get_or_insert(asset_id.id.to_string());
        entity.insert(Name::new(name.clone()));
        entity.insert(local_transform.unwrap_or_default());
        entity.insert(GlobalTransform::identity());

        let entity_id = entity.id();

        // try to hook the parent
        if let Some(parent_id) = runtime_entity.parent.as_ref() {
            if let Some(parent) = asset_to_entity_map.get(parent_id.id()) {
                entity.insert(Parent(parent));
                entity.commands().entity(parent).push_children(&[entity_id]);
            }
        }

        // try to hook the children
        for child_ref in &runtime_entity.children {
            if let Some(child_entity) = asset_to_entity_map.get(child_ref.id()) {
                entity.push_children(&[child_entity]);
            }
        }

        info!(
            "Spawned {}: {} -> ECS id: {:?}| {}",
            Self::TYPENAME,
            asset_id.id,
            entity_id,
            name,
        );
        Some(entity_id)
    }
}

impl AssetToECS for runtime_data::Instance {
    fn create_in_ecs(
        commands: &mut Commands<'_, '_>,
        _instance: &Self,
        asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        _existing_children: Option<&Children>,
    ) -> Option<Entity> {
        let entity = if let Some(entity) = asset_to_entity_map.get(*asset_id) {
            commands.entity(entity)
        } else {
            commands.spawn()
        };

        info!(
            "Spawned {}: {} -> ECS id: {:?}",
            Self::TYPENAME,
            asset_id.id,
            entity.id(),
        );
        Some(entity.id())
    }
}

impl AssetToECS for lgn_graphics_data::runtime::Material {
    fn create_in_ecs(
        commands: &mut Commands<'_, '_>,
        material: &Self,
        asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        _existing_children: Option<&Children>,
    ) -> Option<Entity> {
        let mut entity = if let Some(entity) = asset_to_entity_map.get(*asset_id) {
            commands.entity(entity)
        } else {
            commands.spawn()
        };

        entity.insert(MaterialComponent::new(
            *asset_id,
            material.albedo.clone(),
            material.normal.clone(),
            material.metalness.clone(),
            material.roughness.clone(),
        ));

        info!(
            "Spawned {}: {} -> ECS id: {:?}",
            Self::TYPENAME,
            asset_id.id,
            entity.id(),
        );
        Some(entity.id())
    }
}

impl AssetToECS for lgn_graphics_data::runtime_texture::Texture {
    fn create_in_ecs(
        commands: &mut Commands<'_, '_>,
        texture: &Self,
        asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        _existing_children: Option<&Children>,
    ) -> Option<Entity> {
        let mut entity = if let Some(entity) = asset_to_entity_map.get(*asset_id) {
            commands.entity(entity)
        } else {
            commands.spawn()
        };

        let texture_mips = texture // TODO: Avoid cloning in the future
            .texture_data
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>();

        let texture_data = TextureData::from_slices(&texture_mips);

        let texture_component = TextureComponent::new(
            *asset_id,
            texture.width,
            texture.height,
            texture.format,
            texture.srgb,
            texture_data,
        );

        entity.insert(texture_component);
        info!(
            "Spawned {}: {} -> ECS id: {:?} | width: {}, height: {}, format: {:?}",
            Self::TYPENAME.trim_start_matches("runtime_"),
            asset_id.id,
            entity.id(),
            texture.width,
            texture.height,
            texture.format
        );
        Some(entity.id())
    }
}

impl AssetToECS for lgn_graphics_data::runtime::Model {
    fn create_in_ecs(
        commands: &mut Commands<'_, '_>,
        model: &Self,
        asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        _existing_children: Option<&Children>,
    ) -> Option<Entity> {
        let mut entity = if let Some(entity) = asset_to_entity_map.get(*asset_id) {
            commands.entity(entity)
        } else {
            commands.spawn()
        };

        let mut meshes = Vec::new();
        for mesh in &model.meshes {
            meshes.push(Mesh {
                positions: mesh.positions.clone(),
                normals: if !mesh.normals.is_empty() {
                    Some(mesh.normals.clone())
                } else {
                    None
                },
                tangents: if !mesh.tangents.is_empty() {
                    Some(mesh.tangents.clone())
                } else {
                    None
                },
                tex_coords: if !mesh.tex_coords.is_empty() {
                    Some(mesh.tex_coords.clone())
                } else {
                    None
                },
                indices: if !mesh.indices.is_empty() {
                    Some(mesh.indices.clone())
                } else {
                    None
                },
                colors: if !mesh.colors.is_empty() {
                    Some(mesh.colors.iter().map(|v| Into::into(*v)).collect())
                } else {
                    None
                },
                material_id: mesh.material.clone(),
                bounding_sphere: Mesh::calculate_bounding_sphere(&mesh.positions),
            });
        }
        let model_component = ModelComponent {
            model_id: *asset_id,
            meshes,
        };
        entity.insert(model_component);

        info!(
            "Spawned {}: {} -> ECS id: {:?}",
            Self::TYPENAME.trim_start_matches("runtime_"),
            asset_id.id,
            entity.id(),
        );
        Some(entity.id())
    }
}

impl AssetToECS for lgn_scripting::runtime::Script {
    fn create_in_ecs(
        _commands: &mut Commands<'_, '_>,
        _entity: &Self,
        _asset_id: &ResourceTypeAndId,
        _registry: &Res<'_, Arc<AssetRegistry>>,
        _asset_to_entity_map: &mut ResMut<'_, AssetToEntityMap>,
        _existing_children: Option<&Children>,
    ) -> Option<Entity> {
        None
    }
}
