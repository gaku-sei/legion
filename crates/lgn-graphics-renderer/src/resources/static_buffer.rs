use std::sync::{Arc, Mutex, RwLock};

use lgn_graphics_api::{
    Buffer, BufferCreateFlags, BufferDef, BufferView, BufferViewDef, DeviceContext, GPUViewType,
    IndexBufferBinding, IndexType, MemoryUsage, ResourceUsage, VertexBufferBinding,
};
use lgn_tracing::warn;

use super::{Range, RangeAllocator};
use crate::core::{
    GpuUploadManager, RenderCommand, RenderResources, UploadGPUBuffer, UploadGPUResource,
};

const STATIC_BUFFER_RESOURCE_USAGE: ResourceUsage = ResourceUsage::from_bits_truncate(
    ResourceUsage::AS_SHADER_RESOURCE.bits()
        | ResourceUsage::AS_UNORDERED_ACCESS.bits()
        | ResourceUsage::AS_CONST_BUFFER.bits()
        | ResourceUsage::AS_VERTEX_BUFFER.bits()
        | ResourceUsage::AS_INDEX_BUFFER.bits()
        | ResourceUsage::AS_TRANSFERABLE.bits(),
);

pub struct UnifiedStaticBuffer {
    buffer: Buffer,
    read_only_view: BufferView,
    allocator: UnifiedStaticBufferAllocator,
}

impl UnifiedStaticBuffer {
    pub fn new(device_context: &DeviceContext, virtual_buffer_size: u64) -> Self {
        let element_size = std::mem::size_of::<u32>() as u64;
        let element_count = virtual_buffer_size / element_size;
        let buffer_size = element_count * element_size;

        let buffer = device_context.create_buffer(
            BufferDef {
                size: buffer_size,
                usage_flags: STATIC_BUFFER_RESOURCE_USAGE,
                create_flags: BufferCreateFlags::empty(),
                memory_usage: MemoryUsage::GpuOnly,
                always_mapped: false,
            },
            "UnifiedStaticBuffer",
        );

        let read_only_view =
            buffer.create_view(BufferViewDef::as_byte_address_buffer(element_count, true));

        let allocator = UnifiedStaticBufferAllocator::new(device_context, &buffer);

        Self {
            buffer,
            read_only_view,
            allocator,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn allocator(&self) -> &UnifiedStaticBufferAllocator {
        &self.allocator
    }

    pub fn read_only_view(&self) -> &BufferView {
        &self.read_only_view
    }

    pub fn index_buffer_binding(&self) -> IndexBufferBinding {
        IndexBufferBinding::new(&self.buffer, 0, IndexType::Uint16)
    }
}

pub struct StaticBufferView {
    _allocation: StaticBufferAllocation,
    buffer_view: BufferView,
}

impl StaticBufferView {
    fn new(allocation: &StaticBufferAllocation, view_definition: BufferViewDef) -> Self {
        Self {
            _allocation: allocation.clone(),
            buffer_view: allocation.buffer().create_view(view_definition),
        }
    }

    pub fn buffer_view(&self) -> &BufferView {
        &self.buffer_view
    }
}

struct StaticBufferAllocationInner {
    alloc_range: Range,
    aligned_range: Range,
    allocator: UnifiedStaticBufferAllocator,
    resource_usage: ResourceUsage,
}

#[derive(Clone)]
pub(crate) struct StaticBufferAllocation {
    inner: Arc<StaticBufferAllocationInner>,
}

impl StaticBufferAllocation {
    fn new(
        allocator: &UnifiedStaticBufferAllocator,
        alloc_range: Range,
        aligned_range: Range,
        resource_usage: ResourceUsage,
    ) -> Self {
        Self {
            inner: Arc::new(StaticBufferAllocationInner {
                alloc_range,
                aligned_range,
                allocator: allocator.clone(),
                resource_usage,
            }),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.inner.allocator.buffer
    }

    pub fn byte_offset(&self) -> u64 {
        self.inner.aligned_range.begin()
    }

    pub fn vertex_buffer_binding(&self) -> VertexBufferBinding {
        assert!(self
            .inner
            .resource_usage
            .intersects(ResourceUsage::AS_VERTEX_BUFFER));
        VertexBufferBinding::new(self.buffer(), self.byte_offset())
    }

    pub fn index_buffer_binding(&self, index_type: IndexType) -> IndexBufferBinding {
        assert!(self
            .inner
            .resource_usage
            .intersects(ResourceUsage::AS_INDEX_BUFFER));
        IndexBufferBinding::new(self.buffer(), self.byte_offset(), index_type)
    }

    pub fn create_view(&self, view_definition: BufferViewDef) -> StaticBufferView {
        match view_definition.gpu_view_type {
            GPUViewType::ConstantBuffer => assert!(self
                .inner
                .resource_usage
                .intersects(ResourceUsage::AS_CONST_BUFFER)),
            GPUViewType::ShaderResource => assert!(self
                .inner
                .resource_usage
                .intersects(ResourceUsage::AS_SHADER_RESOURCE)),
            GPUViewType::UnorderedAccess => assert!(self
                .inner
                .resource_usage
                .intersects(ResourceUsage::AS_UNORDERED_ACCESS)),
            GPUViewType::RenderTarget | GPUViewType::DepthStencil => panic!(),
        }

        let view_definition = BufferViewDef {
            byte_offset: self.byte_offset(),
            ..view_definition
        };

        StaticBufferView::new(self, view_definition)
    }
}

impl Drop for StaticBufferAllocationInner {
    fn drop(&mut self) {
        self.allocator.free(self.alloc_range);
    }
}

#[derive(Clone)]
pub struct UnifiedStaticBufferAllocator {
    device_context: DeviceContext,
    buffer: Buffer,
    allocator: Arc<Mutex<RangeAllocator>>,
}

impl UnifiedStaticBufferAllocator {
    pub fn new(device_context: &DeviceContext, buffer: &Buffer) -> Self {
        Self {
            device_context: device_context.clone(),
            buffer: buffer.clone(),
            allocator: Arc::new(Mutex::new(RangeAllocator::new(buffer.definition().size))),
        }
    }

    pub(crate) fn allocate(
        &self,
        required_size: u64,
        resource_usage: ResourceUsage,
    ) -> StaticBufferAllocation {
        assert_eq!(
            ResourceUsage::empty(),
            resource_usage & STATIC_BUFFER_RESOURCE_USAGE.complement()
        );

        let resource_usage = if resource_usage.is_empty() {
            STATIC_BUFFER_RESOURCE_USAGE
        } else {
            resource_usage
        };

        let required_alignment = if resource_usage.intersects(ResourceUsage::AS_CONST_BUFFER) {
            self.device_context
                .device_info()
                .min_uniform_buffer_offset_alignment
        } else {
            self.device_context
                .device_info()
                .min_storage_buffer_offset_alignment
        };

        let required_alignment = 256;

        let alloc_size = lgn_utils::memory::round_size_up_to_alignment_u64(required_size, 256);

        if required_size != alloc_size {
            warn!( "UnifiedStaticBufferAllocator: the segment required size ({} bytes) is less than the allocated size ({} bytes). {} bytes of memory will be wasted", required_size, alloc_size, alloc_size-required_size  );
        }

        let allocator = &mut *self.allocator.lock().unwrap();

        let alloc_range = allocator.allocate(alloc_size).unwrap();
        let aligned_range = Range::from_begin_end(
            alloc_range.begin(),
            // lgn_utils::memory::round_size_up_to_alignment_u64(
            //     alloc_range.begin() as u64,
            //     required_alignment as u64,
            // ),
            alloc_range.end(),
        );

        println!("Alloc {:?}", &alloc_range);

        assert_eq!(alloc_range, aligned_range);
        assert_eq!(aligned_range.begin() % required_alignment, 0);

        StaticBufferAllocation::new(self, alloc_range, aligned_range, resource_usage)
    }

    fn free(&self, range: Range) {
        println!("Free {:?}", &range);
        let allocator = &mut *self.allocator.lock().unwrap();
        allocator.free(range);
    }
}

pub struct UniformGPUData<T> {
    gpu_allocator: UnifiedStaticBufferAllocator,
    allocated_pages: RwLock<Vec<StaticBufferAllocation>>,
    elements_per_page: u64,
    marker: std::marker::PhantomData<T>,
}

impl<T> UniformGPUData<T> {
    pub fn new(gpu_allocator: &UnifiedStaticBufferAllocator, elements_per_page: u64) -> Self {
        Self {
            gpu_allocator: gpu_allocator.clone(),
            allocated_pages: RwLock::new(Vec::new()),
            elements_per_page,
            marker: ::std::marker::PhantomData,
        }
    }

    pub fn ensure_index_allocated(&self, index: u32) -> u64 {
        let index_64 = u64::from(index);
        let element_size = std::mem::size_of::<T>() as u64;
        let elements_per_page = self.elements_per_page;
        let required_pages = (index_64 / elements_per_page) + 1;

        let index_of_page = index_64 / elements_per_page;
        let index_in_page = index_64 % elements_per_page;

        {
            let page_read_access = self.allocated_pages.read().unwrap();
            if page_read_access.len() >= required_pages as usize {
                return page_read_access[index_of_page as usize].byte_offset()
                    + (index_in_page * element_size);
            }
        }

        let mut page_write_access = self.allocated_pages.write().unwrap();

        while (page_write_access.len() as u64) < required_pages {
            let segment_size = elements_per_page * std::mem::size_of::<T>() as u64;
            page_write_access.push(
                self.gpu_allocator
                    .allocate(segment_size, ResourceUsage::AS_SHADER_RESOURCE),
            );
        }

        page_write_access[index_of_page as usize].byte_offset() + (index_in_page * element_size)
    }
}

#[derive(Debug)]
pub struct UpdateUnifiedStaticBufferCommand {
    pub src_buffer: Vec<u8>,
    pub dst_offset: u64,
}

impl RenderCommand for UpdateUnifiedStaticBufferCommand {
    fn execute(self, render_resources: &RenderResources) {
        let mut upload_manager = render_resources.get_mut::<GpuUploadManager>();
        let unified_static_buffer = render_resources.get::<UnifiedStaticBuffer>();
        upload_manager.push(UploadGPUResource::Buffer(UploadGPUBuffer {
            src_data: self.src_buffer,
            dst_buffer: unified_static_buffer.buffer().clone(),
            dst_offset: self.dst_offset,
        }));
    }
}
