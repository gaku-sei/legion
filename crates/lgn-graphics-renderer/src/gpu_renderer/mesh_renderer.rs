use lgn_core::Handle;
use lgn_embedded_fs::embedded_watched_file;
use lgn_graphics_api::{
    BlendState, CompareOp, CullMode, DepthState, DeviceContext, Format, GraphicsPipelineDef,
    PrimitiveTopology, RasterizerState, SampleCount, StencilOp, VertexAttributeRate, VertexLayout,
    VertexLayoutAttribute, VertexLayoutBuffer,
};
use lgn_graphics_cgen_runtime::CGenShaderKey;

use crate::{
    cgen::{
        self,
        cgen_type::{CullingEfficiencyStats, GpuInstanceData},
        shader,
    },
    core::{RenderLayers, RENDER_LAYER_DEPTH, RENDER_LAYER_OPAQUE, RENDER_LAYER_PICKING},
    features::GpuInstanceId,
    resources::{
        GpuBufferWithReadback, MaterialId, PipelineDef, PipelineHandle, PipelineManager,
        ReadbackBuffer, UnifiedStaticBuffer,
    },
};

use super::{RenderElement, RenderLayerBatches, RenderStateSet};

embedded_watched_file!(INCLUDE_BRDF, "gpu/include/brdf.hsh");
embedded_watched_file!(INCLUDE_COMMON, "gpu/include/common.hsh");
embedded_watched_file!(
    INCLUDE_FULLSCREEN_TRIANGLE,
    "gpu/include/fullscreen_triangle.hsh"
);
embedded_watched_file!(INCLUDE_MESH, "gpu/include/mesh.hsh");
embedded_watched_file!(INCLUDE_TRANSFORM, "gpu/include/transform.hsh");
embedded_watched_file!(SHADER_SHADER, "gpu/shaders/shader.hlsl");

// TMP -- what is public here is because they are used in the render graph
pub(crate) struct CullingArgBuffers {
    pub(crate) stats_buffer: GpuBufferWithReadback,
    pub(crate) stats_buffer_readback: Option<Handle<ReadbackBuffer>>,
}

// TODO(jsg): Move this somewhere else to be able to remove this struct entirely.
pub struct MeshRenderer {
    pub(crate) render_layer_batches: Vec<RenderLayerBatches>,

    pub(crate) instance_data_indices: Vec<u32>,
    pub(crate) gpu_instance_data: Vec<GpuInstanceData>,

    pub(crate) culling_buffers: CullingArgBuffers,
    pub(crate) culling_stats: CullingEfficiencyStats,

    tmp_batch_ids: Vec<u32>,
    tmp_pipeline_handles: Vec<PipelineHandle>,
}

impl MeshRenderer {
    pub(crate) fn new(
        device_context: &DeviceContext,
        gpu_heap: &UnifiedStaticBuffer,
        render_layers: &RenderLayers,
        pipeline_manager: &PipelineManager,
    ) -> Self {
        let mut render_layer_batches = render_layers
            .iter()
            .map(|_| RenderLayerBatches::new(gpu_heap, false))
            .collect::<Vec<RenderLayerBatches>>();

        let (tmp_batch_ids, tmp_pipeline_handles) =
            Self::initialize_psos(pipeline_manager, &mut render_layer_batches);

        Self {
            render_layer_batches,
            culling_buffers: CullingArgBuffers {
                stats_buffer: GpuBufferWithReadback::new(
                    device_context,
                    std::mem::size_of::<CullingEfficiencyStats>() as u64,
                ),
                stats_buffer_readback: None,
            },
            culling_stats: CullingEfficiencyStats::default(),
            instance_data_indices: vec![],
            gpu_instance_data: vec![],
            tmp_batch_ids,
            tmp_pipeline_handles,
        }
    }

    fn initialize_psos(
        pipeline_manager: &PipelineManager,
        render_layer_batches: &mut Vec<RenderLayerBatches>,
    ) -> (Vec<u32>, Vec<PipelineHandle>) {
        let mut tmp_batch_ids = Vec::new();
        let mut tmp_pipeline_handles = Vec::new();

        let pipeline_handle = build_depth_pso(pipeline_manager);
        tmp_batch_ids.push(
            render_layer_batches[RENDER_LAYER_DEPTH.index()]
                .register_state_set(&RenderStateSet { pipeline_handle }),
        );
        tmp_pipeline_handles.push(pipeline_handle);

        let need_depth_write =
            !render_layer_batches[RENDER_LAYER_OPAQUE.index()].gpu_culling_enabled();
        let pipeline_handle = build_temp_pso(pipeline_manager, need_depth_write);
        tmp_batch_ids.push(
            render_layer_batches[RENDER_LAYER_OPAQUE.index()]
                .register_state_set(&RenderStateSet { pipeline_handle }),
        );
        tmp_pipeline_handles.push(pipeline_handle);

        let pipeline_handle = build_picking_pso(pipeline_manager);
        tmp_batch_ids.push(
            render_layer_batches[RENDER_LAYER_PICKING.index()]
                .register_state_set(&RenderStateSet { pipeline_handle }),
        );
        tmp_pipeline_handles.push(pipeline_handle);

        (tmp_batch_ids, tmp_pipeline_handles)
    }

    pub(crate) fn get_tmp_pso_handle(&self, layer_id: usize) -> PipelineHandle {
        self.tmp_pipeline_handles[layer_id]
    }

    pub(crate) fn register_material(&mut self, _material_id: MaterialId) {
        for (index, layer) in &mut self.render_layer_batches.iter_mut().enumerate() {
            layer.register_state(0, self.tmp_batch_ids[index]);
        }
    }

    pub(crate) fn register_element(&mut self, element: &RenderElement) {
        let new_index = self.gpu_instance_data.len() as u32;
        let gpu_instance_index = element.gpu_instance_id().index();
        if gpu_instance_index >= self.instance_data_indices.len() as u32 {
            self.instance_data_indices
                .resize(gpu_instance_index as usize + 1, u32::MAX);
        }
        assert!(self.instance_data_indices[gpu_instance_index as usize] == u32::MAX);
        self.instance_data_indices[gpu_instance_index as usize] = new_index;

        let mut instance_data = GpuInstanceData::default();
        instance_data.set_gpu_instance_id(gpu_instance_index.into());
        instance_data.set_state_id(0.into());

        for layer in &mut self.render_layer_batches {
            layer.register_element(0, element);
        }

        self.gpu_instance_data.push(instance_data);

        self.invariant();
    }

    pub(crate) fn unregister_element(&mut self, gpu_instance_id: GpuInstanceId) {
        let gpu_instance_index = gpu_instance_id.index();
        let removed_index = self.instance_data_indices[gpu_instance_index as usize] as usize;
        assert!(removed_index as u32 != u32::MAX);
        self.instance_data_indices[gpu_instance_index as usize] = u32::MAX;

        let removed_instance = self.gpu_instance_data.swap_remove(removed_index as usize);
        let removed_instance_id: u32 = removed_instance.gpu_instance_id().into();
        assert!(gpu_instance_index == removed_instance_id);

        if removed_index < self.gpu_instance_data.len() {
            let moved_instance_id: u32 = self.gpu_instance_data[removed_index as usize]
                .gpu_instance_id()
                .into();
            self.instance_data_indices[moved_instance_id as usize] = removed_index as u32;
        }

        for layer in &mut self.render_layer_batches {
            layer.unregister_element(removed_instance.state_id().into(), gpu_instance_id);
        }

        self.invariant();
    }

    fn invariant(&self) {
        for (instance_idx, slot_idx) in self.instance_data_indices.iter().enumerate() {
            if *slot_idx != u32::MAX {
                let gpu_instance_data = &self.gpu_instance_data[*slot_idx as usize];
                assert!(gpu_instance_data.gpu_instance_id() == (instance_idx as u32).into());
            }
        }
    }
}

fn build_depth_pso(pipeline_manager: &PipelineManager) -> PipelineHandle {
    let root_signature = cgen::pipeline_layout::DepthPipelineLayout::root_signature();

    let mut vertex_layout = VertexLayout::default();
    vertex_layout.attributes[0] = Some(VertexLayoutAttribute {
        format: Format::R32_UINT,
        buffer_index: 0,
        location: 0,
        byte_offset: 0,
    });
    vertex_layout.buffers[0] = Some(VertexLayoutBuffer {
        stride: 4,
        rate: VertexAttributeRate::Instance,
    });

    let depth_state = DepthState {
        depth_test_enable: true,
        depth_write_enable: true,
        depth_compare_op: CompareOp::GreaterOrEqual,
        stencil_test_enable: false,
        stencil_read_mask: 0xFF,
        stencil_write_mask: 0xFF,
        front_depth_fail_op: StencilOp::default(),
        front_stencil_compare_op: CompareOp::Always,
        front_stencil_fail_op: StencilOp::default(),
        front_stencil_pass_op: StencilOp::default(),
        back_depth_fail_op: StencilOp::default(),
        back_stencil_compare_op: CompareOp::Always,
        back_stencil_fail_op: StencilOp::default(),
        back_stencil_pass_op: StencilOp::default(),
    };

    let rasterizer_state = RasterizerState {
        cull_mode: CullMode::Back,
        ..RasterizerState::default()
    };

    let shader = pipeline_manager
        .create_shader(
            cgen::CRATE_ID,
            CGenShaderKey::make(
                cgen::shader::depth_shader::ID,
                cgen::shader::depth_shader::NONE,
            ),
        )
        .unwrap();
    pipeline_manager.register_pipeline(PipelineDef::Graphics(GraphicsPipelineDef {
        shader,
        root_signature: root_signature.clone(),
        vertex_layout,
        blend_state: BlendState::default_alpha_disabled(),
        depth_state,
        rasterizer_state,
        color_formats: vec![],
        sample_count: SampleCount::SampleCount1,
        depth_stencil_format: Some(Format::D32_SFLOAT),
        primitive_topology: PrimitiveTopology::TriangleList,
    }))
}

fn build_temp_pso(pipeline_manager: &PipelineManager, need_depth_write: bool) -> PipelineHandle {
    let root_signature = cgen::pipeline_layout::ShaderPipelineLayout::root_signature();

    let mut vertex_layout = VertexLayout::default();
    vertex_layout.attributes[0] = Some(VertexLayoutAttribute {
        format: Format::R32_UINT,
        buffer_index: 0,
        location: 0,
        byte_offset: 0,
    });
    vertex_layout.buffers[0] = Some(VertexLayoutBuffer {
        stride: 4,
        rate: VertexAttributeRate::Instance,
    });

    let depth_state = DepthState {
        depth_test_enable: true,
        depth_write_enable: need_depth_write,
        depth_compare_op: if need_depth_write {
            CompareOp::GreaterOrEqual
        } else {
            CompareOp::Equal
        },
        stencil_test_enable: false,
        stencil_read_mask: 0xFF,
        stencil_write_mask: 0xFF,
        front_depth_fail_op: StencilOp::default(),
        front_stencil_compare_op: CompareOp::Always,
        front_stencil_fail_op: StencilOp::default(),
        front_stencil_pass_op: StencilOp::default(),
        back_depth_fail_op: StencilOp::default(),
        back_stencil_compare_op: CompareOp::Always,
        back_stencil_fail_op: StencilOp::default(),
        back_stencil_pass_op: StencilOp::default(),
    };

    let rasterizer_state = RasterizerState {
        cull_mode: CullMode::Back,
        ..RasterizerState::default()
    };

    let shader = pipeline_manager
        .create_shader(
            cgen::CRATE_ID,
            CGenShaderKey::make(
                cgen::shader::default_shader::ID,
                cgen::shader::default_shader::NONE,
            ),
        )
        .unwrap();
    pipeline_manager.register_pipeline(PipelineDef::Graphics(GraphicsPipelineDef {
        shader,
        root_signature: root_signature.clone(),
        vertex_layout,
        blend_state: BlendState::default_alpha_disabled(),
        depth_state,
        rasterizer_state,
        color_formats: vec![Format::R16G16B16A16_SFLOAT],
        sample_count: SampleCount::SampleCount1,
        depth_stencil_format: Some(Format::D32_SFLOAT),
        primitive_topology: PrimitiveTopology::TriangleList,
    }))
}

fn build_picking_pso(pipeline_manager: &PipelineManager) -> PipelineHandle {
    let root_signature = cgen::pipeline_layout::PickingPipelineLayout::root_signature();

    let mut vertex_layout = VertexLayout::default();
    vertex_layout.attributes[0] = Some(VertexLayoutAttribute {
        format: Format::R32_UINT,
        buffer_index: 0,
        location: 0,
        byte_offset: 0,
    });
    vertex_layout.buffers[0] = Some(VertexLayoutBuffer {
        stride: 4,
        rate: VertexAttributeRate::Instance,
    });

    let depth_state = DepthState {
        depth_test_enable: false,
        depth_write_enable: false,
        depth_compare_op: CompareOp::default(),
        stencil_test_enable: false,
        stencil_read_mask: 0xFF,
        stencil_write_mask: 0xFF,
        front_depth_fail_op: StencilOp::default(),
        front_stencil_compare_op: CompareOp::Always,
        front_stencil_fail_op: StencilOp::default(),
        front_stencil_pass_op: StencilOp::default(),
        back_depth_fail_op: StencilOp::default(),
        back_stencil_compare_op: CompareOp::Always,
        back_stencil_fail_op: StencilOp::default(),
        back_stencil_pass_op: StencilOp::default(),
    };

    let shader = pipeline_manager
        .create_shader(
            cgen::CRATE_ID,
            CGenShaderKey::make(shader::picking_shader::ID, shader::picking_shader::NONE),
        )
        .unwrap();
    pipeline_manager.register_pipeline(PipelineDef::Graphics(GraphicsPipelineDef {
        shader,
        root_signature: root_signature.clone(),
        vertex_layout,
        blend_state: BlendState::default_alpha_disabled(),
        depth_state,
        rasterizer_state: RasterizerState::default(),
        color_formats: vec![Format::R16G16B16A16_SFLOAT],
        sample_count: SampleCount::SampleCount1,
        depth_stencil_format: None,
        primitive_topology: PrimitiveTopology::TriangleList,
    }))
}
