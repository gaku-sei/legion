use super::*;

use crate::Api;
use core::decimal::DecimalF32;
use std::hash::{Hash, Hasher};

use fnv::FnvHasher;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// General configuration that all APIs will make best effort to respect
#[derive(Default)]
pub struct ApiDef {
    // Don't have anything that's universal across APIs to add here yet
}

#[derive(Clone, Debug, Default)]
pub struct BufferElementData {
    // For storage buffers
    pub element_begin_index: u64,
    pub element_count: u64,
    pub element_stride: u64,
}

/// Used to create a `Buffer`
#[derive(Clone, Debug)]
pub struct BufferDef {
    pub size: u64,
    pub alignment: u32, // May be 0
    pub memory_usage: MemoryUsage,
    pub queue_type: QueueType,
    pub resource_type: ResourceType,
    pub always_mapped: bool,

    // Set to undefined unless texture/typed buffer
    pub format: Format,

    // For storage buffers
    pub elements: BufferElementData,
}

impl Default for BufferDef {
    fn default() -> Self {
        BufferDef {
            size: 0,
            alignment: 0,
            memory_usage: MemoryUsage::Unknown,
            queue_type: QueueType::Graphics,
            resource_type: ResourceType::UNDEFINED,
            elements: Default::default(),
            format: Format::UNDEFINED,
            always_mapped: false,
        }
    }
}

impl BufferDef {
    pub fn verify(&self) {
        assert_ne!(self.size, 0);
    }

    pub fn for_staging_buffer(size: usize, resource_type: ResourceType) -> BufferDef {
        BufferDef {
            size: size as u64,
            alignment: 0,
            memory_usage: MemoryUsage::CpuToGpu,
            queue_type: QueueType::Graphics,
            resource_type,
            elements: Default::default(),
            format: Format::UNDEFINED,
            always_mapped: false,
        }
    }

    pub fn for_staging_buffer_data<T: Copy>(data: &[T], resource_type: ResourceType) -> BufferDef {
        Self::for_staging_buffer(core::memory::slice_size_in_bytes(data), resource_type)
    }

    pub fn for_staging_vertex_buffer(size: usize) -> BufferDef {
        Self::for_staging_buffer(size, ResourceType::VERTEX_BUFFER)
    }

    pub fn for_staging_vertex_buffer_data<T: Copy>(data: &[T]) -> BufferDef {
        Self::for_staging_buffer_data(data, ResourceType::VERTEX_BUFFER)
    }

    pub fn for_staging_index_buffer(size: usize) -> BufferDef {
        Self::for_staging_buffer(size, ResourceType::INDEX_BUFFER)
    }

    pub fn for_staging_index_buffer_data<T: Copy>(data: &[T]) -> BufferDef {
        Self::for_staging_buffer_data(data, ResourceType::INDEX_BUFFER)
    }

    pub fn for_staging_uniform_buffer(size: usize) -> BufferDef {
        Self::for_staging_buffer(size, ResourceType::UNIFORM_BUFFER)
    }

    pub fn for_staging_uniform_buffer_data<T: Copy>(data: &[T]) -> BufferDef {
        Self::for_staging_buffer_data(data, ResourceType::UNIFORM_BUFFER)
    }
}

/// Determines how many dimensions the texture will have.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextureDimensions {
    /// Assume 2D if depth = 1, otherwise 3d
    Auto,
    Dim1D,
    Dim2D,
    Dim3D,
}

impl Default for TextureDimensions {
    fn default() -> Self {
        TextureDimensions::Auto
    }
}

impl TextureDimensions {
    pub fn determine_dimensions(self, extents: Extents3D) -> TextureDimensions {
        match self {
            TextureDimensions::Auto => {
                if extents.depth > 1 {
                    TextureDimensions::Dim3D
                } else {
                    TextureDimensions::Dim2D
                }
            }
            TextureDimensions::Dim1D => {
                assert_eq!(extents.height, 1);
                assert_eq!(extents.depth, 1);
                TextureDimensions::Dim1D
            }
            TextureDimensions::Dim2D => {
                assert_eq!(extents.depth, 1);
                TextureDimensions::Dim2D
            }
            TextureDimensions::Dim3D => TextureDimensions::Dim3D,
        }
    }
}

/// Used to create a `Texture`
#[derive(Clone, Debug)]
pub struct TextureDef {
    pub extents: Extents3D,
    // Corresponds to number of vulkan layers, metal array length, and dx12 array size. Generally
    // should be 1, except set to 6 for cubemaps
    pub array_length: u32,
    pub mip_count: u32,
    pub sample_count: SampleCount,
    pub format: Format,
    pub resource_type: ResourceType,
    // descriptors?
    // pointer to image?
    pub dimensions: TextureDimensions,
}

impl Default for TextureDef {
    fn default() -> Self {
        TextureDef {
            extents: Extents3D {
                width: 0,
                height: 0,
                depth: 0,
            },
            array_length: 1,
            mip_count: 1,
            sample_count: SampleCount::SampleCount1,
            format: Format::UNDEFINED,
            resource_type: ResourceType::TEXTURE,
            dimensions: TextureDimensions::Auto,
        }
    }
}

impl TextureDef {
    pub fn verify(&self) {
        assert!(self.extents.width > 0);
        assert!(self.extents.height > 0);
        assert!(self.extents.depth > 0);
        assert!(self.array_length > 0);
        assert!(self.mip_count > 0);
        assert!(self.mip_count < 2 || self.sample_count == SampleCount::SampleCount1);

        if self.resource_type.contains(ResourceType::TEXTURE_CUBE) {
            assert_eq!(self.array_length % 6, 0);
        }

        // we support only one or the other
        assert!(
            !(self.resource_type.contains(
                ResourceType::RENDER_TARGET_ARRAY_SLICES | ResourceType::RENDER_TARGET_DEPTH_SLICES
            ))
        );

        assert!(
            !(self.format.has_depth()
                && self
                    .resource_type
                    .intersects(ResourceType::TEXTURE_READ_WRITE)),
            "Cannot use depth stencil as UAV"
        );
    }
}

/// Used to create a `CommandPool`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPoolDef {
    /// Set to true if the command buffers allocated from the pool are expected to have very short
    /// lifetimes
    pub transient: bool,
}

/// Used to create a `CommandBuffer`
#[derive(Debug, Clone, PartialEq)]
pub struct CommandBufferDef {
    /// Secondary command buffers are used to encode a single pass on multiple threads
    pub is_secondary: bool,
}

/// Used to create a `Swapchain`
#[derive(Clone, Debug)]
pub struct SwapchainDef {
    pub width: u32,
    pub height: u32,
    pub enable_vsync: bool,
    // image count?
}

/// Describes a single stage within a shader
#[derive(Clone, Debug)]
pub struct ShaderStageDef<A: Api> {
    pub shader_module: A::ShaderModule,
    pub reflection: ShaderStageReflection,
}

impl<A: Api> ShaderStageDef<A> {
    pub fn hash_definition<HasherT: std::hash::Hasher, ShaderModuleHashT: Hash>(
        hasher: &mut HasherT,
        reflection_data: &[&ShaderStageReflection],
        shader_module_hashes: &[ShaderModuleHashT],
    ) {
        assert_eq!(reflection_data.len(), shader_module_hashes.len());
        fn hash_stage<HasherT: std::hash::Hasher, ShaderModuleHashT: Hash>(
            hasher: &mut HasherT,
            stage_flag: ShaderStageFlags,
            reflection_data: &[&ShaderStageReflection],
            shader_module_hashes: &[ShaderModuleHashT],
        ) {
            for (reflection, shader_module_hash) in reflection_data.iter().zip(shader_module_hashes)
            {
                if reflection.shader_stage.intersects(stage_flag) {
                    reflection.shader_stage.hash(hasher);
                    reflection.entry_point_name.hash(hasher);
                    reflection.resources.hash(hasher);
                    shader_module_hash.hash(hasher);
                    break;
                }
            }
        }

        // Hash stages in a deterministic order
        for stage_flag in &super::ALL_SHADER_STAGE_FLAGS {
            hash_stage(hasher, *stage_flag, reflection_data, shader_module_hashes);
        }
    }
}

/// Indicates which immutable sampler is being set
#[derive(Clone, Hash)]
pub enum ImmutableSamplerKey<'a> {
    Name(&'a str),
    Binding(u32, u32),
}

impl<'a> ImmutableSamplerKey<'a> {
    pub fn from_name(name: &'a str) -> ImmutableSamplerKey<'a> {
        ImmutableSamplerKey::Name(name)
    }

    pub fn from_binding(set_index: u32, binding: u32) -> ImmutableSamplerKey<'a> {
        ImmutableSamplerKey::Binding(set_index, binding)
    }
}

/// Describes an immutable sampler key/value pair
pub struct ImmutableSamplers<'a, A: Api> {
    pub key: ImmutableSamplerKey<'a>,
    pub samplers: &'a [A::Sampler],
}

impl<'a, A: Api> ImmutableSamplers<'a, A> {
    pub fn from_name(name: &'a str, samplers: &'a [A::Sampler]) -> ImmutableSamplers<'a, A> {
        ImmutableSamplers {
            key: ImmutableSamplerKey::from_name(name),
            samplers,
        }
    }

    pub fn from_binding(
        set_index: u32,
        binding: u32,
        samplers: &'a [A::Sampler],
    ) -> ImmutableSamplers<'a, A> {
        ImmutableSamplers {
            key: ImmutableSamplerKey::from_binding(set_index, binding),
            samplers,
        }
    }
}

/// Used to create a `RootSignature`
pub struct RootSignatureDef<'a, A: Api> {
    pub shaders: &'a [A::Shader],
    pub immutable_samplers: &'a [ImmutableSamplers<'a, A>],
}

impl<'a, A: Api> RootSignatureDef<'a, A> {
    // The current implementation here is minimal. It will produce different hash values for
    // shader orderings and immutable samplers.
    pub fn hash_definition<
        HasherT: std::hash::Hasher,
        ShaderHashT: Hash,
        ImmutableSamplerHashT: Hash,
    >(
        hasher: &mut HasherT,
        shader_hashes: &[ShaderHashT],
        immutable_sampler_keys: &[ImmutableSamplerKey<'_>],
        immutable_sampler_hashes: &[Vec<ImmutableSamplerHashT>],
    ) {
        // Hash all the shader hashes and xor them together, this keeps them order-independent
        let mut combined_shaders_hash = 0;
        for shader_hash in shader_hashes {
            let mut h = FnvHasher::default();
            shader_hash.hash(&mut h);
            combined_shaders_hash ^= h.finish();
        }

        // Hash all the sampler key/value pairs and xor them together, this keeps them
        // order-independent
        let mut combined_immutable_samplers_hash = 0;
        for (key, samplers) in immutable_sampler_keys.iter().zip(immutable_sampler_hashes) {
            let mut h = FnvHasher::default();
            key.hash(&mut h);
            samplers.hash(&mut h);
            combined_immutable_samplers_hash ^= h.finish();
        }

        // Hash both combined hashes to produce the final hash
        combined_shaders_hash.hash(hasher);
        combined_immutable_samplers_hash.hash(hasher);
    }
}

/// Used to create a `Sampler`
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct SamplerDef {
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub min_filter: FilterType,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub mag_filter: FilterType,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub mip_map_mode: MipMapMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub address_mode_u: AddressMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub address_mode_v: AddressMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub address_mode_w: AddressMode,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub mip_lod_bias: f32,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub max_anisotropy: f32,
    #[cfg_attr(feature = "serde-support", serde(default))]
    pub compare_op: CompareOp,
    //NOTE: Custom hash impl, don't forget to add changes there too!
}

impl Eq for SamplerDef {}

impl Hash for SamplerDef {
    fn hash<H: Hasher>(&self, mut state: &mut H) {
        self.min_filter.hash(&mut state);
        self.mag_filter.hash(&mut state);
        self.mip_map_mode.hash(&mut state);
        self.address_mode_u.hash(&mut state);
        self.address_mode_v.hash(&mut state);
        self.address_mode_w.hash(&mut state);
        DecimalF32(self.mip_lod_bias).hash(&mut state);
        DecimalF32(self.max_anisotropy).hash(&mut state);
        self.compare_op.hash(&mut state);
    }
}

/// Describes an attribute within a VertexLayout
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexLayoutAttribute {
    /// Format of the attribute
    pub format: Format,
    /// Which buffer the attribute is contained in
    pub buffer_index: u32,
    /// Affects what input variable within the shader the attribute is assigned
    pub location: u32,
    /// The byte offset of the attribute within the buffer
    pub byte_offset: u32,

    /// name of the attribute in the shader, only required for GL
    pub gl_attribute_name: Option<String>,
}

/// Describes a buffer that provides vertex attribute data (See VertexLayout)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexLayoutBuffer {
    pub stride: u32,
    pub rate: VertexAttributeRate,
}

/// Describes how vertex attributes are laid out within one or more buffers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexLayout {
    pub attributes: Vec<VertexLayoutAttribute>,
    pub buffers: Vec<VertexLayoutBuffer>,
}

/// Affects depth testing and stencil usage. Commonly used to enable "Z-buffering".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct DepthState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub stencil_test_enable: bool,
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub front_depth_fail_op: StencilOp,
    pub front_stencil_compare_op: CompareOp,
    pub front_stencil_fail_op: StencilOp,
    pub front_stencil_pass_op: StencilOp,
    pub back_depth_fail_op: StencilOp,
    pub back_stencil_compare_op: CompareOp,
    pub back_stencil_fail_op: StencilOp,
    pub back_stencil_pass_op: StencilOp,
}

impl Default for DepthState {
    fn default() -> Self {
        DepthState {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: CompareOp::LessOrEqual,
            stencil_test_enable: false,
            stencil_read_mask: 0xFF,
            stencil_write_mask: 0xFF,
            front_depth_fail_op: Default::default(),
            front_stencil_compare_op: CompareOp::Always,
            front_stencil_fail_op: Default::default(),
            front_stencil_pass_op: Default::default(),
            back_depth_fail_op: Default::default(),
            back_stencil_compare_op: CompareOp::Always,
            back_stencil_fail_op: Default::default(),
            back_stencil_pass_op: Default::default(),
        }
    }
}

/// Affects rasterization, commonly used to enable backface culling or wireframe rendering
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RasterizerState {
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub fill_mode: FillMode,
    pub depth_bias: i32,
    pub depth_bias_slope_scaled: f32,
    pub depth_clamp_enable: bool,
    pub multisample: bool,
    pub scissor: bool,
    // Hash implemented manually below, don't forget to update it!
}

impl Eq for RasterizerState {}

impl Hash for RasterizerState {
    fn hash<H: Hasher>(&self, mut state: &mut H) {
        self.cull_mode.hash(&mut state);
        self.front_face.hash(&mut state);
        self.fill_mode.hash(&mut state);
        self.depth_bias.hash(&mut state);
        DecimalF32(self.depth_bias_slope_scaled).hash(&mut state);
        self.depth_clamp_enable.hash(&mut state);
        self.multisample.hash(&mut state);
        self.scissor.hash(&mut state);
    }
}

impl Default for RasterizerState {
    fn default() -> Self {
        RasterizerState {
            cull_mode: CullMode::None,
            front_face: Default::default(),
            fill_mode: Default::default(),
            depth_bias: 0,
            depth_bias_slope_scaled: 0.0,
            depth_clamp_enable: false,
            multisample: false,
            scissor: false,
        }
    }
}

/// Configures blend state for a particular render target
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct BlendStateRenderTarget {
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub src_factor_alpha: BlendFactor,
    pub dst_factor_alpha: BlendFactor,
    pub blend_op: BlendOp,
    pub blend_op_alpha: BlendOp,
    pub masks: ColorFlags,
}

impl Default for BlendStateRenderTarget {
    fn default() -> Self {
        BlendStateRenderTarget {
            blend_op: BlendOp::Add,
            blend_op_alpha: BlendOp::Add,
            src_factor: BlendFactor::One,
            src_factor_alpha: BlendFactor::One,
            dst_factor: BlendFactor::Zero,
            dst_factor_alpha: BlendFactor::Zero,
            masks: ColorFlags::ALL,
        }
    }
}

impl BlendStateRenderTarget {
    pub fn default_alpha_disabled() -> Self {
        Default::default()
    }

    pub fn default_alpha_enabled() -> Self {
        BlendStateRenderTarget {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            src_factor_alpha: BlendFactor::One,
            dst_factor_alpha: BlendFactor::Zero,
            blend_op: BlendOp::Add,
            blend_op_alpha: BlendOp::Add,
            masks: ColorFlags::ALL,
        }
    }
}

impl BlendStateRenderTarget {
    pub fn blend_enabled(&self) -> bool {
        self.src_factor != BlendFactor::One
            || self.src_factor_alpha != BlendFactor::One
            || self.dst_factor != BlendFactor::Zero
            || self.dst_factor_alpha != BlendFactor::Zero
    }
}

/// Affects the way the result of a pixel shader is blended with a value it will overwrite. Commonly
/// used to enable "alpha-blending".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct BlendState {
    /// Individual blend states for blend targets
    pub render_target_blend_states: Vec<BlendStateRenderTarget>,

    /// Indicates which blend targets to affect. Blend targets with unset bits are left in default
    /// state.
    pub render_target_mask: BlendStateTargets,

    /// If false, `render_target_blend_states[0]` will apply to all render targets indicated by
    /// `render_target_mask`. If true, we index into `render_target_blend_states` based on the
    /// render target's index.
    pub independent_blend: bool,
}

impl BlendState {
    pub fn default_alpha_disabled() -> Self {
        BlendState {
            render_target_blend_states: vec![BlendStateRenderTarget::default_alpha_disabled()],
            render_target_mask: BlendStateTargets::BLEND_STATE_TARGET_ALL,
            independent_blend: false,
        }
    }

    pub fn default_alpha_enabled() -> Self {
        BlendState {
            render_target_blend_states: vec![BlendStateRenderTarget::default_alpha_enabled()],
            render_target_mask: BlendStateTargets::BLEND_STATE_TARGET_ALL,
            independent_blend: false,
        }
    }
}

impl Default for BlendState {
    fn default() -> Self {
        Self::default_alpha_disabled()
    }
}

impl BlendState {
    pub fn verify(&self, color_attachment_count: usize) {
        if !self.independent_blend {
            assert_eq!(self.render_target_blend_states.len(), 1, "If BlendState::independent_blend is false, BlendState::render_target_blend_states must be 1");
        } else {
            assert_eq!(self.render_target_blend_states.len(), color_attachment_count, "If BlendState::independent_blend is true, BlendState::render_target_blend_states length must match color attachment count");
        }
    }
}

/// Used to create a `Pipeline` for graphics operations
#[derive(Debug)]
pub struct GraphicsPipelineDef<'a, A: Api> {
    pub shader: &'a A::Shader,
    pub root_signature: &'a A::RootSignature,
    pub vertex_layout: &'a VertexLayout,
    pub blend_state: &'a BlendState,
    pub depth_state: &'a DepthState,
    pub rasterizer_state: &'a RasterizerState,
    pub primitive_topology: PrimitiveTopology,
    pub color_formats: &'a [Format],
    pub depth_stencil_format: Option<Format>,
    pub sample_count: SampleCount,
    //indirect_commands_enable: bool
}

/// Used to create a `Pipeline` for compute operations
#[derive(Debug)]
pub struct ComputePipelineDef<'a, A: Api> {
    pub shader: &'a A::Shader,
    pub root_signature: &'a A::RootSignature,
}

/// Used to create a `DescriptorSetArray`
pub struct DescriptorSetArrayDef<'a, A: Api> {
    /// The root signature the descriptor set will be based on
    pub root_signature: &'a A::RootSignature,
    /// Which descriptor set to create the descriptor set array for
    pub set_index: u32,
    /// The number of descriptor sets in the array
    pub array_length: usize,
}
