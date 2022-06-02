use lgn_graphics_api::{
    AddressMode, BarrierQueueTransition, BlendState, BufferBarrier, BufferViewDef, CommandBuffer,
    CompareOp, ComputePipelineDef, CullMode, DepthState, DepthStencilClearValue, FilterType,
    Format, GPUViewType, GraphicsPipelineDef, MipMapMode, PlaneSlice, PrimitiveTopology,
    RasterizerState, ResourceState, ResourceUsage, SampleCount, Sampler, SamplerDef, StencilOp,
    TextureDef, VertexAttributeRate, VertexLayout, VertexLayoutAttribute, VertexLayoutBuffer,
    ViewDimension,
};
use lgn_graphics_cgen_runtime::CGenShaderKey;

use lgn_math::Vec2;

use crate::{
    cgen::{self, shader},
    cgen_type::{CullingDebugData, CullingOptions, GpuInstanceData, RenderPassData},
    core::{
        RenderGraphBuilder, RenderGraphContext, RenderGraphExecuteContext, RenderGraphLoadState,
        RenderGraphResourceDef, RenderGraphResourceId, RenderGraphTextureDef,
        RenderGraphTextureViewDef, RenderGraphViewDef, RenderGraphViewId,
    },
    gpu_renderer::{DefaultLayers, GpuInstanceManager, MeshRenderer},
    resources::{PipelineDef, PipelineHandle, PipelineManager, UnifiedStaticBuffer},
    RenderContext,
};

pub struct GpuCullingPass;

#[derive(Clone, Copy)]
struct GPUCullingUserData {
    pso: PipelineHandle,
    draw_count_uav_id: RenderGraphViewId,
    draw_args_uav_id: RenderGraphViewId,
    culled_count_uav_id: RenderGraphViewId,
    culled_args_uav_id: RenderGraphViewId,
    culled_args_buffer_id: RenderGraphResourceId,
    culled_instances_uav_id: RenderGraphViewId,
    first_pass_culled_count_srv_id: RenderGraphViewId, // Used only in second pass
    first_pass_culled_instances_srv_id: RenderGraphViewId, // Used only in second pass
    culling_debug_uav_id: RenderGraphViewId,
    gather_perf_stats: bool,
    hzb_srv_id: RenderGraphViewId,
    hzb_max_lod: u32,
    hzb_extents: Vec2,
}

#[derive(Clone, Copy)]
struct DepthLayerUserData {
    draw_count_buffer_id: RenderGraphResourceId,
    draw_args_buffer_id: RenderGraphResourceId,
}

#[derive(Clone)]
struct HZBUserData {
    pipeline_handle: PipelineHandle,
    mip_sampler: Sampler,
}

impl GpuCullingPass {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build_render_graph<'a>(
        &self,
        builder: RenderGraphBuilder<'a>,
        depth_buffer_id: RenderGraphResourceId,
        depth_view_id: RenderGraphViewId,
        draw_count_buffer_id: RenderGraphResourceId,
        draw_count_buffer_desc: &RenderGraphResourceDef,
        draw_args_buffer_id: RenderGraphResourceId,
        draw_args_buffer_desc: &RenderGraphResourceDef,
        depth_count_buffer_size: u64,
        prev_hzb_texture_desc: &TextureDef,
        prev_hzb_srv_id: RenderGraphViewId,
        current_hzb_texture_desc: &TextureDef,
        current_hzb_id: RenderGraphResourceId,
    ) -> RenderGraphBuilder<'a> {
        let hzb_desc = RenderGraphResourceDef::Texture((*current_hzb_texture_desc).into());
        let mut builder = builder;
        let current_hzb_srv_id =
            builder.declare_view(&RenderGraphViewDef::new_specific_mips_texture_view(
                current_hzb_id,
                0,
                current_hzb_texture_desc.mip_count,
                GPUViewType::ShaderResource,
                false,
            ));

        let hzb_pso = Self::build_hzb_pso(builder.pipeline_manager);
        let mip_sampler = builder.device_context.create_sampler(SamplerDef {
            min_filter: FilterType::Nearest,
            mag_filter: FilterType::Nearest,
            mip_map_mode: MipMapMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mip_lod_bias: 0.0,
            max_anisotropy: 1.0,
            compare_op: CompareOp::Never,
        });

        let culling_psos = Self::build_culling_psos(builder.pipeline_manager);

        let mesh_renderer = builder.render_resources.get::<MeshRenderer>();

        let draw_count_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            draw_count_buffer_id,
            draw_count_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));
        let draw_count_srv_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            draw_count_buffer_id,
            draw_count_buffer_desc,
            GPUViewType::ShaderResource,
        ));

        let draw_args_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            draw_args_buffer_id,
            draw_args_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));

        let culled_count_buffer_desc =
            RenderGraphResourceDef::new_buffer(std::mem::size_of::<u32>() as u64, 1);
        let culled_count_buffer_id =
            builder.declare_buffer("CulledCountBuffer", &culled_count_buffer_desc);
        let culled_count_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culled_count_buffer_id,
            &culled_count_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));
        let culled_count_srv_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culled_count_buffer_id,
            &culled_count_buffer_desc,
            GPUViewType::ShaderResource,
        ));

        let tmp_culled_count_buffer_desc =
            RenderGraphResourceDef::new_buffer(std::mem::size_of::<u32>() as u64, 1);
        let tmp_culled_count_buffer_id =
            builder.declare_buffer("TmpCulledCountBuffer", &tmp_culled_count_buffer_desc);
        let tmp_culled_count_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            tmp_culled_count_buffer_id,
            &tmp_culled_count_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));

        let culled_args_buffer_desc =
            RenderGraphResourceDef::new_buffer(3 * std::mem::size_of::<u32>() as u64, 1);
        let culled_args_buffer_id =
            builder.declare_buffer("CulledArgsBuffer", &culled_args_buffer_desc);
        let culled_args_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culled_args_buffer_id,
            &culled_args_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));
        let culled_args_srv_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culled_args_buffer_id,
            &culled_args_buffer_desc,
            GPUViewType::ShaderResource,
        ));

        let tmp_culled_args_buffer_desc =
            RenderGraphResourceDef::new_buffer(3 * std::mem::size_of::<u32>() as u64, 1);
        let tmp_culled_args_buffer_id =
            builder.declare_buffer("TmpCulledArgsBuffer", &tmp_culled_args_buffer_desc);
        let tmp_culled_args_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            tmp_culled_args_buffer_id,
            &tmp_culled_args_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));

        let culled_instances_buffer_desc = RenderGraphResourceDef::new_buffer(
            std::mem::size_of::<GpuInstanceData>() as u64,
            mesh_renderer.gpu_instance_data.len().max(1) as u64,
        );
        let culled_instances_buffer_id =
            builder.declare_buffer("CulledInstancesBuffer", &culled_instances_buffer_desc);
        let culled_instances_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culled_instances_buffer_id,
            &culled_instances_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));
        let culled_instances_srv_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culled_instances_buffer_id,
            &culled_instances_buffer_desc,
            GPUViewType::ShaderResource,
        ));

        let tmp_culled_instances_buffer_desc = RenderGraphResourceDef::new_buffer(
            std::mem::size_of::<GpuInstanceData>() as u64,
            mesh_renderer.gpu_instance_data.len().max(1) as u64,
        );
        let tmp_culled_instances_buffer_id = builder.declare_buffer(
            "TmpCulledInstancesBuffer",
            &tmp_culled_instances_buffer_desc,
        );
        let tmp_culled_instances_uav_id =
            builder.declare_view(&RenderGraphViewDef::new_buffer_view(
                tmp_culled_instances_buffer_id,
                &tmp_culled_instances_buffer_desc,
                GPUViewType::UnorderedAccess,
            ));

        let culling_debug_buffer_desc = RenderGraphResourceDef::new_buffer(
            std::mem::size_of::<CullingDebugData>() as u64,
            mesh_renderer.gpu_instance_data.len().max(1) as u64,
        );
        let culling_debug_buffer_id =
            builder.declare_buffer("CullingDebug", &culling_debug_buffer_desc);
        let culling_debug_uav_id = builder.declare_view(&RenderGraphViewDef::new_buffer_view(
            culling_debug_buffer_id,
            &culling_debug_buffer_desc,
            GPUViewType::UnorderedAccess,
        ));

        let gather_perf_stats = true;

        builder.add_scope("GPU Culling", |builder| {
            builder
                .add_compute_pass("Culling prev frame HZB", |compute_pass_builder| {
                    let user_data = GPUCullingUserData {
                        pso: culling_psos.0,
                        draw_count_uav_id,
                        draw_args_uav_id,
                        culled_count_uav_id,
                        culled_args_uav_id,
                        culled_args_buffer_id,
                        culled_instances_uav_id,
                        first_pass_culled_count_srv_id: 0, // Unused
                        first_pass_culled_instances_srv_id: 0, // Unused
                        culling_debug_uav_id,
                        gather_perf_stats,
                        hzb_srv_id: prev_hzb_srv_id,
                        hzb_max_lod: prev_hzb_texture_desc.mip_count - 1,
                        hzb_extents: Vec2::new(
                            prev_hzb_texture_desc.extents.width as f32,
                            prev_hzb_texture_desc.extents.height as f32,
                        ),
                    };

                    compute_pass_builder
                        .write(draw_count_uav_id, RenderGraphLoadState::ClearValue(0))
                        .write(draw_args_uav_id, RenderGraphLoadState::ClearValue(0))
                        .write(culled_count_uav_id, RenderGraphLoadState::ClearValue(0))
                        .write(culled_args_uav_id, RenderGraphLoadState::ClearValue(0))
                        .write(culled_instances_uav_id, RenderGraphLoadState::ClearValue(0))
                        .write(culling_debug_uav_id, RenderGraphLoadState::ClearValue(0))
                        .read(prev_hzb_srv_id, RenderGraphLoadState::Load)
                        .execute(move |context, execute_context, cmd_buffer| {
                            Self::execute_culling_pass(
                                context,
                                execute_context,
                                cmd_buffer,
                                user_data,
                                false,
                            );
                        })
                })
                .add_graphics_pass("Depth first pass", |graphics_pass_builder| {
                    let user_data = DepthLayerUserData {
                        draw_count_buffer_id,
                        draw_args_buffer_id,
                    };

                    graphics_pass_builder
                        .depth_stencil(
                            depth_view_id,
                            RenderGraphLoadState::ClearDepthStencil(DepthStencilClearValue {
                                depth: 0.0,
                                stencil: 0,
                            }),
                        )
                        .read(draw_count_srv_id, RenderGraphLoadState::Load)
                        .execute(move |context, execute_context, cmd_buffer| {
                            Self::execute_depth_layer_pass(
                                context,
                                execute_context,
                                cmd_buffer,
                                user_data,
                            );
                        })
                })
                .add_scope("HZB first pass", |builder| {
                    self.build_hzb_render_graph(
                        depth_buffer_id,
                        depth_view_id,
                        current_hzb_id,
                        &hzb_desc,
                        hzb_pso,
                        &mip_sampler,
                        builder,
                    )
                })
                .add_compute_pass("Culling current frame HZB", |compute_pass_builder| {
                    let user_data = GPUCullingUserData {
                        pso: culling_psos.1,
                        draw_count_uav_id,
                        draw_args_uav_id,
                        culled_count_uav_id: tmp_culled_count_uav_id,
                        culled_args_uav_id: tmp_culled_args_uav_id,
                        culled_args_buffer_id,
                        culled_instances_uav_id: tmp_culled_instances_uav_id,
                        first_pass_culled_count_srv_id: culled_count_srv_id,
                        first_pass_culled_instances_srv_id: culled_instances_srv_id,
                        culling_debug_uav_id,
                        gather_perf_stats,
                        hzb_srv_id: current_hzb_srv_id,
                        hzb_max_lod: current_hzb_texture_desc.mip_count - 1,
                        hzb_extents: Vec2::new(
                            current_hzb_texture_desc.extents.width as f32,
                            current_hzb_texture_desc.extents.height as f32,
                        ),
                    };

                    compute_pass_builder
                        .write(draw_count_uav_id, RenderGraphLoadState::Load)
                        .write(draw_args_uav_id, RenderGraphLoadState::Load)
                        .write(tmp_culled_count_uav_id, RenderGraphLoadState::Load)
                        .write(tmp_culled_args_uav_id, RenderGraphLoadState::Load)
                        .write(tmp_culled_instances_uav_id, RenderGraphLoadState::Load)
                        .write(culling_debug_uav_id, RenderGraphLoadState::Load)
                        .read(current_hzb_srv_id, RenderGraphLoadState::Load)
                        .read(culled_count_srv_id, RenderGraphLoadState::Load)
                        .read(culled_args_srv_id, RenderGraphLoadState::Load)
                        .read(culled_instances_srv_id, RenderGraphLoadState::Load)
                        .execute(move |context, execute_context, cmd_buffer| {
                            if depth_count_buffer_size > 0 {
                                // Need to clear the depth instances part of the draw_count buffer
                                // TODO: Should we have a copy pass type? So the graph would manage the resource transitions for us like it does in all other cases?
                                cmd_buffer.cmd_resource_barrier(
                                    &[BufferBarrier {
                                        buffer: context.get_buffer(draw_count_buffer_id),
                                        src_state: ResourceState::UNORDERED_ACCESS,
                                        dst_state: ResourceState::COPY_DST,
                                        queue_transition: BarrierQueueTransition::None,
                                    }],
                                    &[],
                                );

                                let depth_count_size =
                                    depth_count_buffer_size * std::mem::size_of::<u32>() as u64;

                                cmd_buffer.cmd_fill_buffer(
                                    context.get_buffer(draw_count_buffer_id),
                                    0,
                                    depth_count_size,
                                    0,
                                );

                                cmd_buffer.cmd_resource_barrier(
                                    &[BufferBarrier {
                                        buffer: context.get_buffer(draw_count_buffer_id),
                                        src_state: ResourceState::COPY_DST,
                                        dst_state: ResourceState::UNORDERED_ACCESS,
                                        queue_transition: BarrierQueueTransition::None,
                                    }],
                                    &[],
                                );
                            }

                            Self::execute_culling_pass(
                                context,
                                execute_context,
                                cmd_buffer,
                                user_data,
                                true,
                            );
                        })
                })
                .add_graphics_pass("Depth second pass", |graphics_pass_builder| {
                    let user_data = DepthLayerUserData {
                        draw_count_buffer_id,
                        draw_args_buffer_id,
                    };

                    graphics_pass_builder
                        .depth_stencil(depth_view_id, RenderGraphLoadState::Load)
                        .read(draw_count_srv_id, RenderGraphLoadState::Load)
                        .execute(move |context, execute_context, cmd_buffer| {
                            Self::execute_depth_layer_pass(
                                context,
                                execute_context,
                                cmd_buffer,
                                user_data,
                            );
                        })
                })
                .add_scope("HZB second pass", |builder| {
                    self.build_hzb_render_graph(
                        depth_buffer_id,
                        depth_view_id,
                        current_hzb_id,
                        &hzb_desc,
                        hzb_pso,
                        &mip_sampler,
                        builder,
                    )
                })
        })
    }

    fn build_culling_psos(
        pipeline_manager: &mut PipelineManager,
    ) -> (PipelineHandle, PipelineHandle) {
        let root_signature = cgen::pipeline_layout::CullingPipelineLayout::root_signature();

        let shader_first_pass = pipeline_manager
            .create_shader(
                cgen::CRATE_ID,
                CGenShaderKey::make(
                    shader::culling_shader::ID,
                    shader::culling_shader::FIRST_PASS,
                ),
            )
            .unwrap();

        let shader_second_pass = pipeline_manager
            .create_shader(
                cgen::CRATE_ID,
                CGenShaderKey::make(
                    shader::culling_shader::ID,
                    shader::culling_shader::SECOND_PASS,
                ),
            )
            .unwrap();

        (
            pipeline_manager.register_pipeline(PipelineDef::Compute(ComputePipelineDef {
                shader: shader_first_pass,
                root_signature: root_signature.clone(),
            })),
            pipeline_manager.register_pipeline(PipelineDef::Compute(ComputePipelineDef {
                shader: shader_second_pass,
                root_signature: root_signature.clone(),
            })),
        )
    }

    fn build_hzb_pso(pipeline_manager: &PipelineManager) -> PipelineHandle {
        let root_signature = cgen::pipeline_layout::HzbPipelineLayout::root_signature();

        let mut vertex_layout = VertexLayout::default();
        vertex_layout.attributes[0] = Some(VertexLayoutAttribute {
            format: Format::R32G32_SFLOAT,
            buffer_index: 0,
            location: 0,
            byte_offset: 0,
        });
        vertex_layout.attributes[1] = Some(VertexLayoutAttribute {
            format: Format::R32G32_SFLOAT,
            buffer_index: 0,
            location: 1,
            byte_offset: 8,
        });
        vertex_layout.buffers[0] = Some(VertexLayoutBuffer {
            stride: 16,
            rate: VertexAttributeRate::Vertex,
        });

        let depth_state = DepthState {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: CompareOp::Never,
            stencil_test_enable: false,
            stencil_read_mask: 0xFF,
            stencil_write_mask: 0xFF,
            front_depth_fail_op: StencilOp::default(),
            front_stencil_compare_op: CompareOp::default(),
            front_stencil_fail_op: StencilOp::default(),
            front_stencil_pass_op: StencilOp::default(),
            back_depth_fail_op: StencilOp::default(),
            back_stencil_compare_op: CompareOp::default(),
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
                CGenShaderKey::make(cgen::shader::hzb_shader::ID, cgen::shader::hzb_shader::NONE),
            )
            .unwrap();
        pipeline_manager.register_pipeline(PipelineDef::Graphics(GraphicsPipelineDef {
            shader,
            root_signature: root_signature.clone(),
            vertex_layout,
            blend_state: BlendState::default_alpha_disabled(),
            depth_state,
            rasterizer_state,
            color_formats: vec![Format::R32_SFLOAT],
            sample_count: SampleCount::SampleCount1,
            depth_stencil_format: None,
            primitive_topology: PrimitiveTopology::TriangleList,
        }))
    }

    #[allow(clippy::unused_self, clippy::too_many_arguments)]
    fn build_hzb_render_graph<'a>(
        &self,
        depth_buffer_id: RenderGraphResourceId,
        _depth_view_id: RenderGraphViewId,
        hzb_id: RenderGraphResourceId,
        hzb_desc: &RenderGraphResourceDef,
        hzb_pso: PipelineHandle,
        mip_sampler: &Sampler,
        builder: RenderGraphBuilder<'a>,
    ) -> RenderGraphBuilder<'a> {
        let mut builder = builder;

        let depth_view_def = RenderGraphTextureViewDef {
            resource_id: depth_buffer_id,
            gpu_view_type: lgn_graphics_api::GPUViewType::ShaderResource,
            view_dimension: ViewDimension::_2D,
            first_mip: 0,
            mip_count: 1,
            plane_slice: PlaneSlice::Depth,
            first_array_slice: 0,
            array_size: 1,
            read_only: false,
            copy: false,
        };

        let hzb_desc: &RenderGraphTextureDef = hzb_desc.try_into().unwrap();

        let user_data = HZBUserData {
            pipeline_handle: hzb_pso,
            mip_sampler: mip_sampler.clone(),
        };

        for i in 0..hzb_desc.mip_count {
            let mut read_view_def = depth_view_def.clone();
            let read_view_id = if i == 0 {
                builder.declare_view(&RenderGraphViewDef::Texture(read_view_def.clone()))
            } else {
                read_view_def.resource_id = hzb_id;
                read_view_def.plane_slice = PlaneSlice::Default;
                read_view_def.first_mip = i - 1;
                builder.declare_view(&RenderGraphViewDef::Texture(read_view_def.clone()))
            };

            let mut write_view_def = depth_view_def.clone();
            write_view_def.resource_id = hzb_id;
            write_view_def.gpu_view_type = GPUViewType::RenderTarget;
            write_view_def.first_mip = i;
            write_view_def.plane_slice = PlaneSlice::Default;
            let write_view_id =
                builder.declare_view(&RenderGraphViewDef::Texture(write_view_def.clone()));

            let user_data = user_data.clone();

            let pass_name = format!("HZB mip {}", i);
            builder = builder.add_graphics_pass(&pass_name, |mut graphics_pass_builder| {
                graphics_pass_builder = graphics_pass_builder
                    .read(read_view_id, RenderGraphLoadState::Load)
                    .render_target(0, write_view_id, RenderGraphLoadState::DontCare)
                    .execute(move |context, execute_context, cmd_buffer| {
                        let render_context: &mut RenderContext<'_> = execute_context.render_context;

                        let read_view = context.get_texture_view(read_view_id);

                        if let Some(pipeline) = render_context
                            .pipeline_manager
                            .get_pipeline(user_data.pipeline_handle)
                        {
                            cmd_buffer.cmd_bind_pipeline(pipeline);

                            let mut descriptor_set =
                                cgen::descriptor_set::HzbDescriptorSet::default();
                            descriptor_set.set_depth_texture(read_view);
                            descriptor_set.set_depth_sampler(&user_data.mip_sampler);

                            let descriptor_set_handle = render_context.write_descriptor_set(
                                cgen::descriptor_set::HzbDescriptorSet::descriptor_set_layout(),
                                descriptor_set.descriptor_refs(),
                            );
                            cmd_buffer.cmd_bind_descriptor_set_handle(
                                cgen::descriptor_set::HzbDescriptorSet::descriptor_set_layout(),
                                descriptor_set_handle,
                            );

                            #[rustfmt::skip]
                            let vertex_data: [f32; 12] = [0.0, 2.0, 0.0, 2.0,
                                                          0.0, 0.0, 0.0, 0.0,
                                                          2.0, 0.0, 2.0, 0.0];

                            let transient_buffer = render_context
                                .transient_buffer_allocator
                                .copy_data_slice(&vertex_data, ResourceUsage::AS_VERTEX_BUFFER);

                            let vertex_binding = transient_buffer.vertex_buffer_binding();

                            cmd_buffer.cmd_bind_vertex_buffer(0, vertex_binding);

                            cmd_buffer.cmd_draw(3, 0);
                        }
                    });

                graphics_pass_builder
            });
        }

        // We need a dummy pass just to transition the last mip of the current frame HZB to ShaderResource.
        // We expect the whole resource to be in the same state when it comes to be used as previous frame
        // HZB in the next frame.
        let mut last_mip_read_def = depth_view_def;
        last_mip_read_def.resource_id = hzb_id;
        last_mip_read_def.plane_slice = PlaneSlice::Default;
        last_mip_read_def.first_mip = hzb_desc.mip_count - 1;
        let last_mip_read_id =
            builder.declare_view(&RenderGraphViewDef::Texture(last_mip_read_def));

        builder.add_graphics_pass("ChangeLastMipState", |graphics_pass_builder| {
            graphics_pass_builder
                .read(last_mip_read_id, RenderGraphLoadState::Load)
                .execute(|_, _, _| {})
        })
    }

    fn execute_depth_layer_pass(
        context: &RenderGraphContext,
        execute_context: &mut RenderGraphExecuteContext<'_, '_>,
        cmd_buffer: &mut CommandBuffer,
        user_data: DepthLayerUserData,
    ) {
        let render_context = &execute_context.render_context;
        let mesh_renderer = execute_context.render_resources.get::<MeshRenderer>();

        let static_buffer = execute_context
            .render_resources
            .get::<UnifiedStaticBuffer>();

        cmd_buffer.cmd_bind_index_buffer(static_buffer.index_buffer_binding());
        cmd_buffer.cmd_bind_vertex_buffers(
            0,
            &[execute_context
                .render_resources
                .get::<GpuInstanceManager>()
                .vertex_buffer_binding()],
        );

        let layer_id_index = DefaultLayers::Depth as usize;
        mesh_renderer.default_layers[layer_id_index].draw(
            render_context,
            cmd_buffer,
            Some(context.get_buffer(user_data.draw_args_buffer_id)),
            Some(context.get_buffer(user_data.draw_count_buffer_id)),
        );
    }

    fn execute_culling_pass(
        context: &RenderGraphContext,
        execute_context: &mut RenderGraphExecuteContext<'_, '_>,
        cmd_buffer: &mut CommandBuffer,
        user_data: GPUCullingUserData,
        second_pass: bool,
    ) {
        let render_context: &mut RenderContext<'_> = execute_context.render_context;
        let mesh_renderer = execute_context.render_resources.get::<MeshRenderer>();

        if !mesh_renderer.gpu_instance_data.is_empty() {
            if let Some(pipeline) = render_context.pipeline_manager.get_pipeline(user_data.pso) {
                cmd_buffer.cmd_bind_pipeline(pipeline);

                cmd_buffer.cmd_bind_descriptor_set_handle(
                    render_context.frame_descriptor_set().0,
                    render_context.frame_descriptor_set().1,
                );

                cmd_buffer.cmd_bind_descriptor_set_handle(
                    render_context.view_descriptor_set().0,
                    render_context.view_descriptor_set().1,
                );

                let mut culling_descriptor_set =
                    cgen::descriptor_set::CullingDescriptorSet::default();
                culling_descriptor_set
                    .set_draw_count(context.get_buffer_view(user_data.draw_count_uav_id));
                culling_descriptor_set
                    .set_draw_args(context.get_buffer_view(user_data.draw_args_uav_id));
                culling_descriptor_set
                    .set_culled_count(context.get_buffer_view(user_data.culled_count_uav_id));
                culling_descriptor_set
                    .set_culled_args(context.get_buffer_view(user_data.culled_args_uav_id));
                culling_descriptor_set.set_culled_instances(
                    context.get_buffer_view(user_data.culled_instances_uav_id),
                );
                culling_descriptor_set
                    .set_culling_efficiency(mesh_renderer.culling_buffers.stats_buffer.rw_view());

                culling_descriptor_set
                    .set_culling_debug(context.get_buffer_view(user_data.culling_debug_uav_id));

                let mut render_pass_data: Vec<RenderPassData> = vec![];
                for layer in &mesh_renderer.default_layers {
                    let offset_base_va = u32::try_from(layer.offsets_va()).unwrap();

                    let mut pass_data = RenderPassData::default();
                    pass_data.set_offset_base_va(offset_base_va.into());
                    render_pass_data.push(pass_data);
                }

                let gpu_count_allocation = render_context.transient_buffer_allocator.copy_data(
                    &(mesh_renderer.gpu_instance_data.len() as u32),
                    ResourceUsage::AS_SHADER_RESOURCE,
                );

                let gpu_count_view = gpu_count_allocation
                    .to_buffer_view(BufferViewDef::as_structured_buffer_typed::<u32>(1, true));

                let gpu_instance_allocation =
                    render_context.transient_buffer_allocator.copy_data_slice(
                        &mesh_renderer.gpu_instance_data,
                        ResourceUsage::AS_SHADER_RESOURCE,
                    );

                let gpu_instance_view = gpu_instance_allocation.to_buffer_view(
                    BufferViewDef::as_structured_buffer_typed::<GpuInstanceData>(
                        mesh_renderer.gpu_instance_data.len() as u64,
                        true,
                    ),
                );

                let render_pass_allocation = render_context
                    .transient_buffer_allocator
                    .copy_data_slice(&render_pass_data, ResourceUsage::AS_SHADER_RESOURCE);

                let render_pass_view = render_pass_allocation.to_buffer_view(
                    BufferViewDef::as_structured_buffer_typed::<RenderPassData>(
                        render_pass_data.len() as u64,
                        true,
                    ),
                );

                if second_pass {
                    culling_descriptor_set.set_gpu_instance_count(
                        context.get_buffer_view(user_data.first_pass_culled_count_srv_id),
                    );
                    culling_descriptor_set.set_gpu_instance_data(
                        context.get_buffer_view(user_data.first_pass_culled_instances_srv_id),
                    );
                } else {
                    culling_descriptor_set.set_gpu_instance_count(gpu_count_view);
                    culling_descriptor_set.set_gpu_instance_data(gpu_instance_view);
                }
                culling_descriptor_set.set_render_pass_data(render_pass_view);

                culling_descriptor_set
                    .set_hzb_texture(context.get_texture_view(user_data.hzb_srv_id));

                let culling_descriptor_set_handle = render_context.write_descriptor_set(
                    cgen::descriptor_set::CullingDescriptorSet::descriptor_set_layout(),
                    culling_descriptor_set.descriptor_refs(),
                );

                cmd_buffer.cmd_bind_descriptor_set_handle(
                    cgen::descriptor_set::CullingDescriptorSet::descriptor_set_layout(),
                    culling_descriptor_set_handle,
                );

                let mut options = CullingOptions::empty();
                if user_data.gather_perf_stats {
                    options |= CullingOptions::GATHER_PERF_STATS;
                }

                let mut culling_constant_data = cgen::cgen_type::CullingPushConstantData::default();
                culling_constant_data.set_first_render_pass(0.into());
                culling_constant_data.set_num_render_passes((render_pass_data.len() as u32).into());
                culling_constant_data.set_hzb_max_lod(user_data.hzb_max_lod.into());
                culling_constant_data.set_hzb_pixel_extents(user_data.hzb_extents.into());
                culling_constant_data.set_options(options);

                cmd_buffer.cmd_push_constant_typed(&culling_constant_data);

                if second_pass {
                    cmd_buffer.cmd_dispatch_indirect(
                        context.get_buffer(user_data.culled_args_buffer_id),
                        0,
                    );
                } else {
                    cmd_buffer.cmd_dispatch(
                        (mesh_renderer.gpu_instance_data.len() as u32 + 255) / 256,
                        1,
                        1,
                    );
                }
            }
        }
    }
}