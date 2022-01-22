use lgn_graphics_api::{prelude::*, MAX_DESCRIPTOR_SET_LAYOUTS};
use lgn_pso_compiler::{CompileParams, EntryPoint, HlslCompiler, ShaderSource, TargetProfile};
use lgn_renderer::{components::RenderSurface, RenderContext};
use lgn_tracing::span_fn;

use super::Resolution;

struct ResolutionDependentResources {
    resolution: Resolution,
    yuv_images: Vec<(Texture, Texture, Texture)>,
    yuv_image_uavs: Vec<(TextureView, TextureView, TextureView)>,
    copy_yuv_images: Vec<Texture>,
}

impl ResolutionDependentResources {
    fn new(
        device_context: &DeviceContext,
        render_frame_count: u32,
        resolution: Resolution,
    ) -> Result<Self, anyhow::Error> {
        let mut yuv_images = Vec::with_capacity(render_frame_count as usize);
        let mut yuv_image_uavs = Vec::with_capacity(render_frame_count as usize);
        let mut copy_yuv_images = Vec::with_capacity(render_frame_count as usize);
        for _ in 0..render_frame_count {
            let mut yuv_plane_def = TextureDef {
                extents: Extents3D {
                    width: resolution.width,
                    height: resolution.height,
                    depth: 1,
                },
                array_length: 1,
                mip_count: 1,
                format: Format::R8_UNORM,
                mem_usage: MemoryUsage::GpuOnly,
                usage_flags: ResourceUsage::AS_UNORDERED_ACCESS | ResourceUsage::AS_TRANSFERABLE,
                resource_flags: ResourceFlags::empty(),
                tiling: TextureTiling::Optimal,
            };

            let y_image = device_context.create_texture(&yuv_plane_def)?;
            yuv_plane_def.extents.width /= 2;
            yuv_plane_def.extents.height /= 2;
            let u_image = device_context.create_texture(&yuv_plane_def)?;
            let v_image = device_context.create_texture(&yuv_plane_def)?;

            let yuv_plane_uav_def = TextureViewDef {
                gpu_view_type: GPUViewType::UnorderedAccess,
                view_dimension: ViewDimension::_2D,
                first_mip: 0,
                mip_count: 1,
                plane_slice: PlaneSlice::Default,
                first_array_slice: 0,
                array_size: 1,
            };

            let y_image_uav = y_image.create_view(&yuv_plane_uav_def)?;
            let u_image_uav = u_image.create_view(&yuv_plane_uav_def)?;
            let v_image_uav = v_image.create_view(&yuv_plane_uav_def)?;

            let copy_yuv_image = device_context.create_texture(&TextureDef {
                extents: Extents3D {
                    width: resolution.width,
                    height: resolution.height,
                    depth: 1,
                },
                array_length: 1,
                mip_count: 1,
                format: Format::G8_B8_R8_3PLANE_420_UNORM,
                mem_usage: MemoryUsage::GpuToCpu,
                usage_flags: ResourceUsage::AS_TRANSFERABLE,
                resource_flags: ResourceFlags::empty(),
                tiling: TextureTiling::Linear,
            })?;

            yuv_images.push((y_image, u_image, v_image));
            yuv_image_uavs.push((y_image_uav, u_image_uav, v_image_uav));
            copy_yuv_images.push(copy_yuv_image);
        }

        Ok(Self {
            resolution,
            yuv_images,
            yuv_image_uavs,
            copy_yuv_images,
        })
    }
}

pub struct RgbToYuvConverter {
    render_frame_count: u32,
    resolution_dependent_resources: ResolutionDependentResources,
    root_signature: RootSignature,
    pipeline: Pipeline,
}

impl RgbToYuvConverter {
    pub fn new(
        shader_compiler: &HlslCompiler,
        device_context: &DeviceContext,
        resolution: Resolution,
    ) -> anyhow::Result<Self> {
        shader_compiler
            .filesystem()
            .add_mount_point("streamer", env!("CARGO_MANIFEST_DIR"))?;

        let shader_build_result = shader_compiler.compile(&CompileParams {
            shader_source: ShaderSource::Path("crate://streamer/shaders/rgb2yuv.hlsl".to_string()),
            global_defines: Vec::new(),
            entry_points: vec![EntryPoint {
                defines: Vec::new(),
                name: "cs_main".to_owned(),
                target_profile: TargetProfile::Compute,
            }],
        })?;

        let compute_shader_module = device_context.create_shader_module(
            ShaderPackage::SpirV(shader_build_result.spirv_binaries[0].bytecode.clone())
                .module_def(),
        )?;

        let shader = device_context.create_shader(vec![ShaderStageDef {
            entry_point: "cs_main".to_owned(),
            shader_stage: ShaderStageFlags::COMPUTE,
            shader_module: compute_shader_module,
            // reflection: shader_build_result.reflection_info.clone().unwrap(),
        }]);

        let mut descriptor_set_layouts = Vec::new();
        for set_index in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
            let shader_resources: Vec<_> = shader_build_result
                .pipeline_reflection
                .shader_resources
                .iter()
                .filter(|x| x.set_index as usize == set_index)
                .collect();

            if !shader_resources.is_empty() {
                let descriptor_defs = shader_resources
                    .iter()
                    .map(|sr| DescriptorDef {
                        name: sr.name.clone(),
                        binding: sr.binding,
                        shader_resource_type: sr.shader_resource_type,
                        array_size: sr.element_count,
                    })
                    .collect();

                let def = DescriptorSetLayoutDef {
                    frequency: set_index as u32,
                    descriptor_defs,
                };
                let descriptor_set_layout =
                    device_context.create_descriptorset_layout(&def).unwrap();
                descriptor_set_layouts.push(descriptor_set_layout);
            }
        }

        let root_signature_def = RootSignatureDef {
            descriptor_set_layouts: descriptor_set_layouts.clone(),
            push_constant_def: None,
        };

        let root_signature = device_context.create_root_signature(&root_signature_def)?;

        let pipeline = device_context.create_compute_pipeline(&ComputePipelineDef {
            shader: &shader,
            root_signature: &root_signature,
        })?;

        ////////////////////////////////////////////////////////////////////////////////

        let render_frame_count = 1u32;
        let resolution_dependent_resources =
            ResolutionDependentResources::new(device_context, render_frame_count, resolution)?;

        Ok(Self {
            render_frame_count: 1,
            resolution_dependent_resources,
            root_signature,
            pipeline,
        })
    }

    pub fn resize(
        &mut self,
        device_context: &DeviceContext,
        resolution: Resolution,
    ) -> anyhow::Result<bool> {
        if resolution != self.resolution_dependent_resources.resolution {
            self.resolution_dependent_resources = ResolutionDependentResources::new(
                device_context,
                self.render_frame_count,
                resolution,
            )?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[span_fn]
    pub fn convert(
        &mut self,
        render_context: &RenderContext<'_>,
        render_surface: &mut RenderSurface,
        yuv: &mut [u8],
    ) -> anyhow::Result<()> {
        let render_frame_idx = 0;
        let mut cmd_buffer = render_context.alloc_command_buffer();
        render_surface.transition_to(&cmd_buffer, ResourceState::SHADER_RESOURCE);
        {
            let yuv_images = &self.resolution_dependent_resources.yuv_images[render_frame_idx];
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    &yuv_images.0,
                    ResourceState::COPY_SRC,
                    ResourceState::UNORDERED_ACCESS,
                )],
            );
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    &yuv_images.1,
                    ResourceState::COPY_SRC,
                    ResourceState::UNORDERED_ACCESS,
                )],
            );
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    &yuv_images.2,
                    ResourceState::COPY_SRC,
                    ResourceState::UNORDERED_ACCESS,
                )],
            );

            cmd_buffer.bind_pipeline(&self.pipeline);

            let descriptor_set_layout = &self
                .pipeline
                .root_signature()
                .definition()
                .descriptor_set_layouts[0];

            let mut descriptor_set_writer =
                render_context.alloc_descriptor_set(descriptor_set_layout);
            descriptor_set_writer
                .set_descriptors_by_name(
                    "hdr_image",
                    &[DescriptorRef::TextureView(
                        render_surface.shader_resource_view(),
                    )],
                )
                .unwrap();

            let yuv_images_views =
                &self.resolution_dependent_resources.yuv_image_uavs[render_frame_idx];

            descriptor_set_writer
                .set_descriptors_by_name(
                    "y_image",
                    &[DescriptorRef::TextureView(&yuv_images_views.0)],
                )
                .unwrap();
            descriptor_set_writer
                .set_descriptors_by_name(
                    "u_image",
                    &[DescriptorRef::TextureView(&yuv_images_views.1)],
                )
                .unwrap();
            descriptor_set_writer
                .set_descriptors_by_name(
                    "v_image",
                    &[DescriptorRef::TextureView(&yuv_images_views.2)],
                )
                .unwrap();

            let device_context = render_context.renderer().device_context();
            let descriptor_set_handle = descriptor_set_writer.flush(device_context);

            cmd_buffer.bind_descriptor_set_handle_deprecated(
                PipelineType::Compute,
                &self.root_signature,
                descriptor_set_layout.definition().frequency,
                descriptor_set_handle,
            );
            cmd_buffer.dispatch(
                ((self.resolution_dependent_resources.resolution.width + 7) / 8) as u32,
                ((self.resolution_dependent_resources.resolution.height + 7) / 8) as u32,
                1,
            );
        }

        let copy_texture_yuv =
            &self.resolution_dependent_resources.copy_yuv_images[render_frame_idx];
        {
            let yuv_images = &self.resolution_dependent_resources.yuv_images[render_frame_idx];
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    &yuv_images.0,
                    ResourceState::UNORDERED_ACCESS,
                    ResourceState::COPY_SRC,
                )],
            );
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    &yuv_images.1,
                    ResourceState::UNORDERED_ACCESS,
                    ResourceState::COPY_SRC,
                )],
            );
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    &yuv_images.2,
                    ResourceState::UNORDERED_ACCESS,
                    ResourceState::COPY_SRC,
                )],
            );
            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    copy_texture_yuv,
                    ResourceState::COMMON,
                    ResourceState::COPY_DST,
                )],
            );

            let mut copy_extents = copy_texture_yuv.definition().extents;
            assert_eq!(copy_texture_yuv.definition().extents, copy_extents);
            cmd_buffer.copy_image(
                &yuv_images.0,
                copy_texture_yuv,
                &CmdCopyTextureParams {
                    src_state: ResourceState::COPY_SRC,
                    dst_state: ResourceState::COPY_DST,
                    src_offset: Offset3D { x: 0, y: 0, z: 0 },
                    dst_offset: Offset3D { x: 0, y: 0, z: 0 },
                    src_mip_level: 0,
                    dst_mip_level: 0,
                    src_array_slice: 0,
                    dst_array_slice: 0,
                    src_plane_slice: PlaneSlice::Default,
                    dst_plane_slice: PlaneSlice::Plane0,
                    extent: copy_extents,
                },
            );

            copy_extents.width /= 2;
            copy_extents.height /= 2;
            cmd_buffer.copy_image(
                &yuv_images.1,
                copy_texture_yuv,
                &CmdCopyTextureParams {
                    src_state: ResourceState::COPY_SRC,
                    dst_state: ResourceState::COPY_DST,
                    src_offset: Offset3D { x: 0, y: 0, z: 0 },
                    dst_offset: Offset3D { x: 0, y: 0, z: 0 },
                    src_mip_level: 0,
                    dst_mip_level: 0,
                    src_array_slice: 0,
                    dst_array_slice: 0,
                    src_plane_slice: PlaneSlice::Default,
                    dst_plane_slice: PlaneSlice::Plane1,
                    extent: copy_extents,
                },
            );

            cmd_buffer.copy_image(
                &yuv_images.2,
                copy_texture_yuv,
                &CmdCopyTextureParams {
                    src_state: ResourceState::COPY_SRC,
                    dst_state: ResourceState::COPY_DST,
                    src_offset: Offset3D { x: 0, y: 0, z: 0 },
                    dst_offset: Offset3D { x: 0, y: 0, z: 0 },
                    src_mip_level: 0,
                    dst_mip_level: 0,
                    src_array_slice: 0,
                    dst_array_slice: 0,
                    src_plane_slice: PlaneSlice::Default,
                    dst_plane_slice: PlaneSlice::Plane2,
                    extent: copy_extents,
                },
            );

            cmd_buffer.resource_barrier(
                &[],
                &[TextureBarrier::state_transition(
                    copy_texture_yuv,
                    ResourceState::COPY_DST,
                    ResourceState::COMMON,
                )],
            );
        }

        //
        // Present the image
        //

        let wait_sem = render_surface.sema();
        let graphics_queue = render_context.graphics_queue();

        graphics_queue.submit(&mut [cmd_buffer.finalize()], &[wait_sem], &[], None);

        graphics_queue.wait_for_queue_idle()?;
        let (mut width, mut height) = (
            copy_texture_yuv.definition().extents.width as usize,
            copy_texture_yuv.definition().extents.height as usize,
        );

        let sub_resource = copy_texture_yuv.map_texture(PlaneSlice::Plane0)?;
        let mut amount_copied = 0;
        for y in 0..height {
            yuv[amount_copied..(y + 1) * width].copy_from_slice(
                &sub_resource.data[y * sub_resource.row_pitch as usize
                    ..(y * sub_resource.row_pitch as usize) + width],
            );
            amount_copied += width;
        }
        copy_texture_yuv.unmap_texture();

        let sub_resource = copy_texture_yuv.map_texture(PlaneSlice::Plane1)?;
        width /= 2;
        height /= 2;
        let start = amount_copied;
        for y in 0..height {
            yuv[amount_copied..start + (y + 1) * width].copy_from_slice(
                &sub_resource.data[y * sub_resource.row_pitch as usize
                    ..(y * sub_resource.row_pitch as usize) + width],
            );
            amount_copied += width;
        }
        copy_texture_yuv.unmap_texture();

        let sub_resource = copy_texture_yuv.map_texture(PlaneSlice::Plane2)?;
        let start = amount_copied;
        for y in 0..height {
            yuv[amount_copied..start + (y + 1) * width].copy_from_slice(
                &sub_resource.data[y * sub_resource.row_pitch as usize
                    ..(y * sub_resource.row_pitch as usize) + width],
            );
            amount_copied += width;
        }
        copy_texture_yuv.unmap_texture();
        assert_eq!(amount_copied, yuv.len());
        Ok(())
    }
}