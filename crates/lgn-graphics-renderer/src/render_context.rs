use lgn_graphics_api::{
    CommandBuffer, DescriptorRef, DescriptorSetHandle, DescriptorSetLayout, DeviceContext,
};

use crate::{
    resources::{
        CommandBufferHandle, DescriptorPoolHandle, PipelineManager, TransientBufferAllocator,
        TransientCommandBufferAllocator, UnifiedStaticBuffer,
    },
    GraphicsQueue,
};

// pub(crate) type TransientBufferAllocatorHandle = Handle<TransientBufferAllocator>;

pub struct RenderContext<'frame> {
    // renderer: &'frame Renderer,
    pub device_context: &'frame DeviceContext,
    pub graphics_queue: &'frame GraphicsQueue,
    pub descriptor_pool: &'frame DescriptorPoolHandle,
    pub pipeline_manager: &'frame PipelineManager,
    pub transient_commandbuffer_allocator: &'frame mut TransientCommandBufferAllocator,
    pub transient_buffer_allocator: &'frame mut TransientBufferAllocator,
    pub static_buffer: &'frame UnifiedStaticBuffer,
    // tmp
    persistent_descriptor_set: Option<(&'frame DescriptorSetLayout, DescriptorSetHandle)>,
    frame_descriptor_set: Option<(&'frame DescriptorSetLayout, DescriptorSetHandle)>,
    view_descriptor_set: Option<(&'frame DescriptorSetLayout, DescriptorSetHandle)>,
}

impl<'frame> RenderContext<'frame> {
    pub fn new(
        device_context: &'frame DeviceContext,
        graphics_queue: &'frame GraphicsQueue,
        descriptor_pool: &'frame DescriptorPoolHandle,
        pipeline_manager: &'frame PipelineManager,
        transient_commandbuffer_allocator: &'frame mut TransientCommandBufferAllocator,
        transient_buffer_allocator: &'frame mut TransientBufferAllocator,
        static_buffer: &'frame UnifiedStaticBuffer,
    ) -> Self {
        Self {
            device_context,
            graphics_queue,
            descriptor_pool,
            pipeline_manager,
            transient_commandbuffer_allocator,
            transient_buffer_allocator,
            static_buffer,
            persistent_descriptor_set: None,
            frame_descriptor_set: None,
            view_descriptor_set: None,
        }
    }

    #[allow(clippy::todo)]
    pub fn write_descriptor_set(
        &self,
        layout: &DescriptorSetLayout,
        descriptors: &[DescriptorRef],
    ) -> DescriptorSetHandle {
        self.descriptor_pool
            .write_descriptor_set(layout, descriptors)
    }

    pub fn persistent_descriptor_set(&self) -> (&DescriptorSetLayout, DescriptorSetHandle) {
        self.persistent_descriptor_set.unwrap()
    }

    pub fn set_persistent_descriptor_set(
        &mut self,
        layout: &'frame DescriptorSetLayout,
        handle: DescriptorSetHandle,
    ) {
        self.persistent_descriptor_set = Some((layout, handle));
    }

    pub fn frame_descriptor_set(&self) -> (&DescriptorSetLayout, DescriptorSetHandle) {
        self.frame_descriptor_set.unwrap()
    }

    pub fn set_frame_descriptor_set(
        &mut self,
        layout: &'frame DescriptorSetLayout,
        handle: DescriptorSetHandle,
    ) {
        self.frame_descriptor_set = Some((layout, handle));
    }

    pub fn view_descriptor_set(&self) -> (&DescriptorSetLayout, DescriptorSetHandle) {
        self.view_descriptor_set.unwrap()
    }

    pub fn set_view_descriptor_set(
        &mut self,
        layout: &'frame DescriptorSetLayout,
        handle: DescriptorSetHandle,
    ) {
        self.view_descriptor_set = Some((layout, handle));
    }

    pub fn bind_default_descriptor_sets(&self, cmd_buffer: &mut CommandBuffer) {
        cmd_buffer.cmd_bind_descriptor_set_handle(
            self.persistent_descriptor_set.unwrap().0,
            self.persistent_descriptor_set.unwrap().1,
        );
        cmd_buffer.cmd_bind_descriptor_set_handle(
            self.frame_descriptor_set.unwrap().0,
            self.frame_descriptor_set.unwrap().1,
        );
        cmd_buffer.cmd_bind_descriptor_set_handle(
            self.view_descriptor_set.unwrap().0,
            self.view_descriptor_set.unwrap().1,
        );
    }
}
