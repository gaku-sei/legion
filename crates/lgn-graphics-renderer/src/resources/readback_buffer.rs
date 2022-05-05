use std::slice;

use lgn_core::Handle;
use lgn_graphics_api::{
    BarrierQueueTransition, Buffer, BufferBarrier, BufferCopy, BufferCreateFlags, BufferDef,
    BufferView, BufferViewDef, DeviceContext, MemoryUsage, ResourceState, ResourceUsage,
};

use crate::hl_gfx_api::HLCommandBuffer;

use super::GpuSafePool;

pub(crate) struct ReadbackBuffer {
    buffer: Buffer,
    // allocation: MemoryAllocation,
    cpu_frame_for_results: u64,
}

impl ReadbackBuffer {
    pub(crate) fn new(device_context: &DeviceContext, size: u64) -> Self {
        let buffer = device_context.create_buffer(
            BufferDef {
                size,
                usage_flags: ResourceUsage::AS_TRANSFERABLE,
                create_flags: BufferCreateFlags::empty(),
                memory_usage: MemoryUsage::GpuToCpu,
                always_mapped: false,
            },
            "ReadbackBuffer",
        );

        Self {
            buffer,
            cpu_frame_for_results: u64::MAX,
        }
    }

    pub(crate) fn read_gpu_data<T: Sized, F: FnMut(&[T])>(
        &self,
        offset: usize,
        mut count: usize,
        cpu_frame_no: u64,
        mut f: F,
    ) {
        if cpu_frame_no == u64::MAX
            || (self.cpu_frame_for_results != u64::MAX
                && self.cpu_frame_for_results == cpu_frame_no)
        {
            let mapping_info = self.buffer.map_buffer();
            #[allow(unsafe_code)]
            unsafe {
                let element_size = std::mem::size_of::<T>();
                let byte_offset = offset * element_size;
                if count == usize::MAX {
                    count = self.buffer.definition().size as usize / element_size;
                }
                let byte_count = count * element_size;

                assert!(byte_offset + byte_count <= self.buffer.definition().size as usize);
                f(slice::from_raw_parts(
                    mapping_info.data_ptr().add(offset) as *const T,
                    count,
                ));
            }
            self.buffer.unmap_buffer();
        }
    }

    pub(crate) fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(crate) fn sent_to_gpu(&mut self, cpu_frame_for_results: u64) {
        self.cpu_frame_for_results = cpu_frame_for_results;
    }
}

// impl OnFrameEventHandler for ReadbackBuffer {
//     fn on_begin_frame(&mut self) {}

//     fn on_end_frame(&mut self) {}
// }

pub(crate) struct GpuBufferWithReadback {
    buffer: Buffer,
    rw_view: BufferView,
    readback_pool: GpuSafePool<ReadbackBuffer>,
}

impl GpuBufferWithReadback {
    pub(crate) fn new(device_context: &DeviceContext, element_size: u64) -> Self {
        let buffer = device_context.create_buffer(
            BufferDef {
                size: element_size,
                usage_flags: ResourceUsage::AS_SHADER_RESOURCE
                    | ResourceUsage::AS_UNORDERED_ACCESS
                    | ResourceUsage::AS_TRANSFERABLE,
                create_flags: BufferCreateFlags::empty(),
                memory_usage: MemoryUsage::GpuOnly,
                always_mapped: false,
            },
            "GpuReadbackBuffer",
        );

        let rw_view =
            buffer.create_view(BufferViewDef::as_structured_buffer(1, element_size, false));

        Self {
            buffer,
            rw_view,
            readback_pool: GpuSafePool::new(3),
        }
    }

    pub(crate) fn begin_readback(
        &mut self,
        device_context: &DeviceContext,
    ) -> Handle<ReadbackBuffer> {
        self.readback_pool.begin_frame(|_| ());
        self.readback_pool.acquire_or_create(|| {
            ReadbackBuffer::new(device_context, self.buffer.definition().size)
        })
    }

    pub(crate) fn end_readback(&mut self, buffer: Handle<ReadbackBuffer>) {
        self.readback_pool.release(buffer);
        self.readback_pool.end_frame(|_| ());
    }

    pub(crate) fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(crate) fn rw_view(&self) -> &BufferView {
        &self.rw_view
    }

    pub(crate) fn clear_buffer(&self, cmd_buffer: &mut HLCommandBuffer) {
        cmd_buffer.resource_barrier(
            &[BufferBarrier {
                buffer: self.buffer(),
                src_state: ResourceState::UNORDERED_ACCESS,
                dst_state: ResourceState::COPY_DST,
                queue_transition: BarrierQueueTransition::None,
            }],
            &[],
        );

        cmd_buffer.fill_buffer(self.buffer(), 0, !0, 0);

        cmd_buffer.resource_barrier(
            &[BufferBarrier {
                buffer: self.buffer(),
                src_state: ResourceState::COPY_DST,
                dst_state: ResourceState::UNORDERED_ACCESS,
                queue_transition: BarrierQueueTransition::None,
            }],
            &[],
        );
    }

    pub(crate) fn copy_buffer_to_readback(
        &self,
        cmd_buffer: &mut HLCommandBuffer,
        readback: &Handle<ReadbackBuffer>,
    ) {
        cmd_buffer.resource_barrier(
            &[BufferBarrier {
                buffer: self.buffer(),
                src_state: ResourceState::UNORDERED_ACCESS,
                dst_state: ResourceState::COPY_SRC,
                queue_transition: BarrierQueueTransition::None,
            }],
            &[],
        );

        cmd_buffer.copy_buffer_to_buffer(
            self.buffer(),
            readback.buffer(),
            &[BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: self.buffer().definition().size,
            }],
        );

        cmd_buffer.resource_barrier(
            &[BufferBarrier {
                buffer: self.buffer(),
                src_state: ResourceState::COPY_SRC,
                dst_state: ResourceState::UNORDERED_ACCESS,
                queue_transition: BarrierQueueTransition::None,
            }],
            &[],
        );
    }
}
