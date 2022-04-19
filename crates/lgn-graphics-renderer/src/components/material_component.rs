use lgn_data_runtime::Handle;
use lgn_ecs::prelude::*;
use lgn_graphics_data::{
    runtime::BinTextureReferenceType,
    runtime::{Material, SamplerData},
    Color,
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask(f32),
    Blend(f32),
}

impl Eq for AlphaMode {}

impl Default for AlphaMode {
    fn default() -> Self {
        Self::Opaque
    }
}

#[derive(Default, Clone)]
pub struct MaterialData {
    pub albedo_texture: Option<BinTextureReferenceType>,
    pub base_albedo: Color,
    pub normal_texture: Option<BinTextureReferenceType>,
    pub metalness_texture: Option<BinTextureReferenceType>,
    pub base_metalness: f32,
    pub reflectance: f32,
    pub roughness_texture: Option<BinTextureReferenceType>,
    pub base_roughness: f32,
    pub alpha_mode: AlphaMode,
    pub sampler_data: Option<SamplerData>,
}

#[derive(Component)]
pub struct MaterialComponent {
    pub resource: Handle<Material>,
    pub material_data: MaterialData,
}

impl MaterialComponent {
    pub fn new(
        resource: Handle<Material>,
        albedo_texture: Option<BinTextureReferenceType>,
        normal_texture: Option<BinTextureReferenceType>,
        metalness_texture: Option<BinTextureReferenceType>,
        roughness_texture: Option<BinTextureReferenceType>,
        sampler_data: Option<SamplerData>,
    ) -> Self {
        Self {
            resource,
            material_data: MaterialData {
                albedo_texture,
                base_albedo: Color::from((204, 204, 204)),
                normal_texture,
                metalness_texture,
                base_metalness: 0.0,
                reflectance: 0.5,
                roughness_texture,
                base_roughness: 0.4,
                alpha_mode: AlphaMode::Opaque,
                sampler_data,
            },
        }
    }
}
