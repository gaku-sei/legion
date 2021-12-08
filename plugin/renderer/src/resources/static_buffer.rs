use std::sync::{Arc, Mutex};

use lgn_graphics_api::{
    BarrierQueueTransition, Buffer, BufferAllocation, BufferBarrier, BufferCopy, BufferDef,
    BufferView, BufferViewDef, DeviceContext, MemoryPagesAllocation, PagedBufferAllocation, Queue,
    QueueType, ResourceCreation, ResourceState, ResourceUsage, Semaphore,
};
use lgn_math::Mat4;

use parking_lot::RwLockReadGuard;

use crate::{RenderContext, RenderHandle};

use super::{RangeAllocator, SparseBindingManager, TransientPagedBuffer};

pub(crate) struct UnifiedStaticBufferInner {
    buffer: Buffer,
    segment_allocator: RangeAllocator,
    binding_manager: SparseBindingManager,
    page_size: u64,
    read_only_view: BufferView,
    job_blocks: Vec<UniformGPUDataUploadJobBlock>,
}

#[derive(Clone)]
pub struct UnifiedStaticBuffer {
    inner: Arc<Mutex<UnifiedStaticBufferInner>>,
}

impl UnifiedStaticBuffer {
    pub fn new(device_context: &DeviceContext, virtual_buffer_size: u64) -> Self {
        let buffer_def = BufferDef {
            size: virtual_buffer_size,
            queue_type: QueueType::Graphics,
            usage_flags: ResourceUsage::AS_SHADER_RESOURCE | ResourceUsage::AS_TRANSFERABLE,
            creation_flags: ResourceCreation::SPARSE_BINDING,
        };

        let buffer = device_context.create_buffer(&buffer_def);
        let required_alignment = buffer.required_alignment();

        assert!(virtual_buffer_size % required_alignment as u64 == 0);

        let ro_view_def = BufferViewDef::as_byte_address_buffer(buffer.definition(), true);
        let read_only_view = BufferView::from_buffer(&buffer, &ro_view_def).unwrap();

        Self {
            inner: Arc::new(Mutex::new(UnifiedStaticBufferInner {
                buffer,
                segment_allocator: RangeAllocator::new(virtual_buffer_size),
                binding_manager: SparseBindingManager::new(),
                page_size: required_alignment,
                read_only_view,
                job_blocks: Vec::new(),
            })),
        }
    }

    pub fn allocate_segment(&self, segment_size: u64) -> PagedBufferAllocation {
        let inner = &mut *self.inner.lock().unwrap();

        let page_size = inner.page_size;
        let page_count =
            lgn_utils::memory::round_size_up_to_alignment_u64(segment_size, page_size) / page_size;
        let alloc_size = page_count * page_size;

        let location = inner.segment_allocator.allocate(alloc_size).unwrap();
        let allocation = MemoryPagesAllocation::for_sparse_buffer(
            inner.buffer.device_context(),
            &inner.buffer,
            page_count,
        );

        let paged_allocation = PagedBufferAllocation {
            buffer: inner.buffer.clone(),
            memory: allocation,
            range: location,
        };

        inner
            .binding_manager
            .add_sparse_binding(paged_allocation.clone());

        paged_allocation
    }

    pub fn free_segment(&self, segment: PagedBufferAllocation) {
        let inner = &mut *self.inner.lock().unwrap();

        inner.segment_allocator.free(segment.range);
        inner.binding_manager.add_sparse_unbinding(segment);
    }

    pub fn add_update_job_block(&self, job_blocks: &mut Vec<UniformGPUDataUploadJobBlock>) {
        let inner = &mut *self.inner.lock().unwrap();

        inner.job_blocks.append(job_blocks);
    }

    pub fn flush_updater(
        &self,
        prev_frame_semaphore: &Semaphore,
        unbind_semaphore: &Semaphore,
        bind_semaphore: &Semaphore,
        render_context: &mut RenderContext<'_>,
        graphics_queue: &RwLockReadGuard<'_, Queue>,
    ) {
        let inner = &mut *self.inner.lock().unwrap();

        let last_semaphore = inner.binding_manager.commmit_sparse_bindings(
            graphics_queue,
            prev_frame_semaphore,
            unbind_semaphore,
            bind_semaphore,
        );

        let cmd_buffer = render_context.acquire_cmd_buffer(QueueType::Graphics);

        cmd_buffer.begin().unwrap();

        cmd_buffer
            .cmd_resource_barrier(
                &[BufferBarrier {
                    buffer: &inner.buffer,
                    src_state: ResourceState::SHADER_RESOURCE,
                    dst_state: ResourceState::COPY_DST,
                    queue_transition: BarrierQueueTransition::None,
                }],
                &[],
            )
            .unwrap();

        for job in &inner.job_blocks {
            cmd_buffer.cmd_copy_buffer_to_buffer(
                &job.upload_allocation.buffer,
                &inner.buffer,
                &job.upload_jobs,
            );
        }
        inner.job_blocks.clear();

        cmd_buffer
            .cmd_resource_barrier(
                &[BufferBarrier {
                    buffer: &inner.buffer,
                    src_state: ResourceState::COPY_DST,
                    dst_state: ResourceState::SHADER_RESOURCE,
                    queue_transition: BarrierQueueTransition::None,
                }],
                &[],
            )
            .unwrap();

        cmd_buffer.end().unwrap();

        let mut wait_sems = Vec::new();
        if last_semaphore.signal_available() {
            wait_sems.push(last_semaphore);
            last_semaphore.set_signal_available(false);
        }

        graphics_queue
            .submit(&[&cmd_buffer], &wait_sems, &[], None)
            .unwrap();

        render_context.release_cmd_buffer(cmd_buffer);
    }

    pub fn read_only_view(&self) -> BufferView {
        let inner = self.inner.lock().unwrap();

        inner.read_only_view.clone()
    }
}

pub struct EntityTransforms {
    pub world: Mat4,
}

pub struct UniformGPUData<T> {
    static_baffer: UnifiedStaticBuffer,
    allocated_pages: Vec<PagedBufferAllocation>,
    page_size: u64,
    element_size: u64,
    marker: std::marker::PhantomData<T>,
}

impl<T> UniformGPUData<T> {
    pub fn new(static_baffer: &UnifiedStaticBuffer, min_page_size: u64) -> Self {
        let page = static_baffer.allocate_segment(min_page_size);
        let page_size = page.size();
        Self {
            static_baffer: static_baffer.clone(),
            allocated_pages: vec![page],
            page_size,
            element_size: std::mem::size_of::<T>() as u64,
            marker: ::std::marker::PhantomData,
        }
    }

    pub fn ensure_index_allocated(&mut self, index: u32) -> u64 {
        let index_64 = u64::from(index);
        let elements_per_page = self.page_size / self.element_size;
        let required_pages = (index_64 / elements_per_page) + 1;

        while (self.allocated_pages.len() as u64) < required_pages {
            self.allocated_pages
                .push(self.static_baffer.allocate_segment(self.page_size));
        }

        let index_of_page = index_64 / elements_per_page;
        let index_in_page = index_64 % elements_per_page;

        self.allocated_pages[index_of_page as usize].offset() + (index_in_page * self.element_size)
    }
}

pub type TestStaticBuffer = RenderHandle<UniformGPUData<EntityTransforms>>;

pub struct UniformGPUDataUploadJobBlock {
    upload_allocation: BufferAllocation,
    upload_jobs: Vec<BufferCopy>,
    offset: u64,
}

impl UniformGPUDataUploadJobBlock {
    fn new(upload_allocation: BufferAllocation) -> Self {
        Self {
            upload_allocation,
            upload_jobs: Vec::new(),
            offset: 0,
        }
    }

    fn add_update_jobs<T>(&mut self, data: &[T], dst_offset: u64) -> bool {
        let upload_size_in_bytes = lgn_utils::memory::slice_size_in_bytes(data) as u64;
        if self.offset + upload_size_in_bytes <= self.upload_allocation.size() {
            let src = data.as_ptr().cast::<u8>();
            let upload_offset = self.upload_allocation.offset() + self.offset;
            {
                #[allow(unsafe_code)]
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        src,
                        self.upload_allocation
                            .memory
                            .mapped_ptr()
                            .add(upload_offset as usize),
                        upload_size_in_bytes as usize,
                    );
                }
            }

            for i in 0..data.len() as u64 {
                let data_size = std::mem::size_of::<T>() as u64;
                self.upload_jobs.push(BufferCopy {
                    src_offset: upload_offset,
                    dst_offset: dst_offset + (i * data_size),
                    size: data_size,
                });
                self.offset += data_size;
            }
            true
        } else {
            false
        }
    }
}

pub struct UniformGPUDataUpdater {
    paged_buffer: TransientPagedBuffer,
    job_blocks: Vec<UniformGPUDataUploadJobBlock>,
    block_size: u64,
}

impl UniformGPUDataUpdater {
    pub fn new(paged_buffer: TransientPagedBuffer, block_size: u64) -> Self {
        Self {
            paged_buffer,
            job_blocks: Vec::new(),
            block_size,
        }
    }

    pub fn add_update_jobs<T>(&mut self, data: &[T], dst_offset: u64) {
        while self.job_blocks.is_empty()
            || !self
                .job_blocks
                .last_mut()
                .unwrap()
                .add_update_jobs(data, dst_offset)
        {
            self.job_blocks.push(UniformGPUDataUploadJobBlock::new(
                self.paged_buffer.allocate_page(self.block_size),
            ));
        }
    }

    pub fn job_blocks(&mut self) -> &mut Vec<UniformGPUDataUploadJobBlock> {
        &mut self.job_blocks
    }
}