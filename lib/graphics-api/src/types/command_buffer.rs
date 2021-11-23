#![allow(clippy::too_many_lines)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "vulkan")]
use crate::backends::vulkan::VulkanCommandBuffer;
use crate::{Buffer, CommandPool, DescriptorSetHandle, Pipeline, Texture};
use crate::{
    BufferBarrier, CmdBlitParams, CmdCopyBufferToTextureParams, CmdCopyTextureParams,
    ColorRenderTargetBinding, CommandBufferDef, DepthStencilRenderTargetBinding, DeviceContext,
    GfxResult, IndexBufferBinding, QueueType, RootSignature, TextureBarrier, VertexBufferBinding,
};

pub struct CommandBuffer {
    device_context: DeviceContext,
    queue_type: QueueType,
    queue_family_index: u32,
    has_active_renderpass: AtomicBool,

    #[cfg(feature = "vulkan")]
    platform_command_buffer: VulkanCommandBuffer,
}

impl CommandBuffer {
    pub fn new(
        device_context: &DeviceContext,
        command_pool: &CommandPool,
        command_buffer_def: &CommandBufferDef,
    ) -> GfxResult<Self> {
        #[cfg(feature = "vulkan")]
        let platform_command_buffer = VulkanCommandBuffer::new(command_pool, command_buffer_def)
            .map_err(|e| {
                log::error!("Error creating command buffer {:?}", e);
                ash::vk::Result::ERROR_UNKNOWN
            })?;

        Ok(Self {
            device_context: device_context.clone(),
            queue_type: command_pool.queue_type(),
            queue_family_index: command_pool.queue_family_index(),
            has_active_renderpass: AtomicBool::new(false),
            #[cfg(any(feature = "vulkan"))]
            platform_command_buffer,
        })
    }

    #[cfg(feature = "vulkan")]
    pub(crate) fn platform_command_buffer(&self) -> &VulkanCommandBuffer {
        &self.platform_command_buffer
    }

    pub fn begin(&self) -> GfxResult<()> {
        #[cfg(not(any(feature = "vulkan")))]
        unimplemented!();

        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.begin(&self.device_context)
    }

    pub fn end(&self) -> GfxResult<()> {
        if self.has_active_renderpass.load(Ordering::Relaxed) {
            #[cfg(any(feature = "vulkan"))]
            self.platform_command_buffer
                .cmd_end_render_pass(&self.device_context);
            self.has_active_renderpass.store(false, Ordering::Relaxed);
        }

        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .end_command_buffer(&self.device_context)?;

        Ok(())
    }

    pub fn return_to_pool(&self) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .return_to_pool(&self.device_context);
        Ok(())
    }

    pub fn cmd_begin_render_pass(
        &self,
        color_targets: &[ColorRenderTargetBinding<'_>],
        depth_target: &Option<DepthStencilRenderTargetBinding<'_>>,
    ) -> GfxResult<()> {
        if self.has_active_renderpass.load(Ordering::Relaxed) {
            self.cmd_end_render_pass()?;
        }

        if color_targets.is_empty() && depth_target.is_none() {
            return Err("No color or depth target supplied to cmd_begin_render_pass".into());
        }

        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_begin_render_pass(
            &self.device_context,
            self.queue_type,
            self.queue_family_index,
            color_targets,
            depth_target,
        )?;

        self.has_active_renderpass.store(true, Ordering::Relaxed);

        Ok(())
    }

    pub fn cmd_end_render_pass(&self) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .cmd_end_render_pass(&self.device_context);
        self.has_active_renderpass.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub fn cmd_set_viewport(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        depth_min: f32,
        depth_max: f32,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_set_viewport(
            &self.device_context,
            x,
            y,
            width,
            height,
            depth_min,
            depth_max,
        );
        Ok(())
    }

    pub fn cmd_set_scissor(&self, x: u32, y: u32, width: u32, height: u32) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .cmd_set_scissor(&self.device_context, x, y, width, height);
        Ok(())
    }

    pub fn cmd_set_stencil_reference_value(&self, value: u32) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .cmd_set_stencil_reference_value(&self.device_context, value);
        Ok(())
    }

    pub fn cmd_bind_pipeline(&self, pipeline: &Pipeline) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .cmd_bind_pipeline(&self.device_context, pipeline);
        Ok(())
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[VertexBufferBinding<'_>],
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_bind_vertex_buffers(
            &self.device_context,
            first_binding,
            bindings,
        );
        Ok(())
    }

    pub fn cmd_bind_index_buffer(&self, binding: &IndexBufferBinding<'_>) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .cmd_bind_index_buffer(&self.device_context, binding);
        Ok(())
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RootSignature,
        set_index: u32,
        descriptor_set_handle: DescriptorSetHandle,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_bind_descriptor_set_handle(
            &self.device_context,
            root_signature,
            set_index,
            descriptor_set_handle,
        );
        Ok(())
    }

    pub fn cmd_push_constants<T: Sized>(
        &self,
        root_signature: &RootSignature,
        constants: &T,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_push_constants(
            &self.device_context,
            root_signature.platform_root_signature(),
            constants,
        );
        Ok(())
    }

    pub fn cmd_draw(&self, vertex_count: u32, first_vertex: u32) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer
            .cmd_draw(&self.device_context, vertex_count, first_vertex);
        Ok(())
    }

    pub fn cmd_draw_instanced(
        &self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_draw_instanced(
            &self.device_context,
            vertex_count,
            first_vertex,
            instance_count,
            first_instance,
        );
        Ok(())
    }

    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_draw_indexed(
            &self.device_context,
            index_count,
            first_index,
            vertex_offset,
        );
        Ok(())
    }

    pub fn cmd_draw_indexed_instanced(
        &self,
        index_count: u32,
        first_index: u32,
        instance_count: u32,
        first_instance: u32,
        vertex_offset: i32,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_draw_indexed_instanced(
            &self.device_context,
            index_count,
            first_index,
            instance_count,
            first_instance,
            vertex_offset,
        );
        Ok(())
    }

    pub fn cmd_dispatch(
        &self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_dispatch(
            &self.device_context,
            group_count_x,
            group_count_y,
            group_count_z,
        );
        Ok(())
    }

    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[BufferBarrier<'_>],
        texture_barriers: &[TextureBarrier<'_>],
    ) -> GfxResult<()> {
        assert!(
            !self.has_active_renderpass.load(Ordering::Relaxed),
            "cmd_resource_barrier may not be called if inside render pass"
        );
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_resource_barrier(
            &self.device_context,
            self.queue_type,
            self.queue_family_index,
            buffer_barriers,
            texture_barriers,
        );
        Ok(())
    }

    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &Buffer,
        dst_buffer: &Buffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_copy_buffer_to_buffer(
            &self.device_context,
            src_buffer,
            dst_buffer,
            src_offset,
            dst_offset,
            size,
        );
        Ok(())
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &Buffer,
        dst_texture: &Texture,
        params: &CmdCopyBufferToTextureParams,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_copy_buffer_to_texture(
            &self.device_context,
            src_buffer,
            dst_texture,
            params,
        );
        Ok(())
    }

    pub fn cmd_blit_texture(
        &self,
        src_texture: &Texture,
        dst_texture: &Texture,
        params: &CmdBlitParams,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_blit_texture(
            &self.device_context,
            src_texture,
            dst_texture,
            params,
        );
        Ok(())
    }

    pub fn cmd_copy_image(
        &self,
        src_texture: &Texture,
        dst_texture: &Texture,
        params: &CmdCopyTextureParams,
    ) -> GfxResult<()> {
        #[cfg(any(feature = "vulkan"))]
        self.platform_command_buffer.cmd_copy_image(
            &self.device_context,
            src_texture,
            dst_texture,
            params,
        );
        Ok(())
    }
}