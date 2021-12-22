use crate::components::RenderSurface;
use crate::debug_display::{DebugDisplay, DebugPrimitiveType};
use crate::hl_gfx_api::HLCommandBuffer;
use crate::static_mesh_render_data::StaticMeshRenderData;
use crate::{RenderContext, Renderer};
use lgn_graphics_api::prelude::*;
use lgn_math::{Mat4, Vec3};
use lgn_transform::components::Transform;

pub struct DebugDisplayPass {
    root_signature: RootSignature,
    pipeline: Pipeline,
}

impl DebugDisplayPass {
    pub fn new(renderer: &Renderer) -> Self {
        let device_context = renderer.device_context();
        let (shader, root_signature) =
            renderer.prepare_vs_ps(String::from("crate://renderer/shaders/debug_display.hlsl"));
        //
        // Pipeline state
        //
        let vertex_layout = VertexLayout {
            attributes: vec![VertexLayoutAttribute {
                format: Format::R32G32B32_SFLOAT,
                buffer_index: 0,
                location: 0,
                byte_offset: 0,
                gl_attribute_name: Some("pos".to_owned()),
            }],
            buffers: vec![VertexLayoutBuffer {
                stride: 12,
                rate: VertexAttributeRate::Vertex,
            }],
        };

        let depth_state = DepthState {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare_op: CompareOp::Less,
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

        let pipeline = device_context
            .create_graphics_pipeline(&GraphicsPipelineDef {
                shader: &shader,
                root_signature: &root_signature,
                vertex_layout: &vertex_layout,
                blend_state: &BlendState::default_alpha_enabled(),
                depth_state: &depth_state,
                rasterizer_state: &RasterizerState {
                    fill_mode: FillMode::Wireframe,
                    ..RasterizerState::default()
                },
                color_formats: &[Format::R16G16B16A16_SFLOAT],
                sample_count: SampleCount::SampleCount1,
                depth_stencil_format: Some(Format::D32_SFLOAT),
                primitive_topology: PrimitiveTopology::TriangleList,
            })
            .unwrap();

        Self {
            root_signature,
            pipeline,
        }
    }

    pub fn render(
        &self,
        render_context: &RenderContext<'_>,
        cmd_buffer: &HLCommandBuffer<'_>,
        render_surface: &RenderSurface,
        debug_display: &mut DebugDisplay,
        camera_transform: &Transform,
    ) {
        cmd_buffer.begin_render_pass(
            &[ColorRenderTargetBinding {
                texture_view: render_surface.render_target_view(),
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
                clear_value: ColorClearValue::default(),
            }],
            &Some(DepthStencilRenderTargetBinding {
                texture_view: render_surface.depth_stencil_texture_view(),
                depth_load_op: LoadOp::Load,
                stencil_load_op: LoadOp::DontCare,
                depth_store_op: StoreOp::Store,
                stencil_store_op: StoreOp::DontCare,
                clear_value: DepthStencilClearValue {
                    depth: 1.0,
                    stencil: 0,
                },
            }),
        );

        cmd_buffer.bind_pipeline(&self.pipeline);

        let descriptor_set_layout = &self
            .pipeline
            .root_signature()
            .definition()
            .descriptor_set_layouts[0];

        let fov_y_radians: f32 = 45.0;
        let width = render_surface.extents().width() as f32;
        let height = render_surface.extents().height() as f32;
        let aspect_ratio: f32 = width / height;
        let z_near: f32 = 0.01;
        let z_far: f32 = 100.0;
        let projection_matrix = Mat4::perspective_lh(fov_y_radians, aspect_ratio, z_near, z_far);

        let view_matrix = Mat4::look_at_lh(
            camera_transform.translation,
            camera_transform.translation + camera_transform.forward(),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let transient_allocator = render_context.transient_buffer_allocator();

        debug_display.render_primitives(|primitive| {
            let mesh_data = match primitive.primitive_type {
                DebugPrimitiveType::Cube => StaticMeshRenderData::new_cube(0.1),
                DebugPrimitiveType::Arrow { dir } => {
                    StaticMeshRenderData::new_arrow(Vec3::new(0.0, 0.0, 0.0), dir)
                }
            };

            let mut sub_allocation = transient_allocator.copy_data(
                &mesh_data
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|(idx, ..)| idx % 14 < 3)
                    .map(|(_idx, v)| *v)
                    .collect::<Vec<f32>>(),
                ResourceUsage::AS_VERTEX_BUFFER,
            );

            cmd_buffer.bind_buffer_suballocation_as_vertex_buffer(0, &sub_allocation);

            let color: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);

            let world = Mat4::from_translation(primitive.pos).transpose();
            let mut push_constant_data: [f32; 52] = [0.0; 52];
            world.write_cols_to_slice(&mut push_constant_data[0..]);
            view_matrix.write_cols_to_slice(&mut push_constant_data[16..]);
            projection_matrix.write_cols_to_slice(&mut push_constant_data[32..]);
            push_constant_data[48] = color.0;
            push_constant_data[49] = color.1;
            push_constant_data[50] = color.2;
            push_constant_data[51] = 1.0;

            sub_allocation =
                transient_allocator.copy_data(&push_constant_data, ResourceUsage::AS_CONST_BUFFER);

            let const_buffer_view = sub_allocation.const_buffer_view();

            let mut descriptor_set_writer =
                render_context.alloc_descriptor_set(descriptor_set_layout);

            descriptor_set_writer
                .set_descriptors_by_name(
                    "const_data",
                    &[DescriptorRef::BufferView(&const_buffer_view)],
                )
                .unwrap();

            let descriptor_set_handle =
                descriptor_set_writer.flush(render_context.renderer().device_context());

            cmd_buffer.bind_descriptor_set_handle(
                PipelineType::Graphics,
                &self.root_signature,
                descriptor_set_layout.definition().frequency,
                descriptor_set_handle,
            );

            cmd_buffer.draw((mesh_data.num_vertices()) as u32, 0);
        });

        debug_display.clear_display_lists();

        cmd_buffer.end_render_pass();
    }
}