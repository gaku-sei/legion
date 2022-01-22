#![allow(unsafe_code)]

use anyhow::Result;
use lgn_graphics_api::Queue;
use lgn_graphics_api::{
    ApiDef, BufferView, DescriptorHeap, DescriptorHeapDef, DeviceContext, Fence, FenceStatus,
    GfxApi, QueueType, Semaphore, Shader, ShaderPackage, ShaderStageDef, ShaderStageFlags,
};
use lgn_pso_compiler::{
    CompileParams, EntryPoint, FileSystem, HlslCompiler, ShaderSource, TargetProfile,
};
use lgn_tracing::span_fn;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};

use crate::cgen::cgen_type::{DirectionalLight, OmniDirectionalLight, SpotLight};
use crate::memory::{BumpAllocator, BumpAllocatorHandle};
use crate::resources::{
    CommandBufferPool, CommandBufferPoolHandle, CpuPool, DescriptorPool, DescriptorPoolHandle,
    GpuSafePool, TransformStaticsBuffer, TransientPagedBuffer, UnifiedStaticBuffer, UniformGPUData,
    UniformGPUDataUploadJobBlock,
};
use crate::{cgen, RenderContext, RenderHandle};

pub struct Renderer {
    frame_idx: usize,
    render_frame_idx: usize,
    num_render_frames: usize,
    prev_frame_sems: Vec<Semaphore>,
    sparse_unbind_sems: Vec<Semaphore>,
    sparse_bind_sems: Vec<Semaphore>,
    frame_fences: Vec<Fence>,
    graphics_queue: RwLock<Queue>,
    descriptor_heap: DescriptorHeap,
    command_buffer_pools: Mutex<GpuSafePool<CommandBufferPool>>,
    descriptor_pools: Mutex<GpuSafePool<DescriptorPool>>,
    transient_buffer: TransientPagedBuffer,
    static_buffer: UnifiedStaticBuffer,
    transforms_data: TransformStaticsBuffer,
    omnidirectional_lights_data: OmniDirectionalLightsStaticBuffer,
    directional_lights_data: DirectionalLightsStaticBuffer,
    spotlights_data: SpotLightsStaticBuffer,
    bump_allocator_pool: Mutex<CpuPool<BumpAllocator>>,
    shader_compiler: HlslCompiler,
    // This should be last, as it must be destroyed last.
    api: GfxApi,
}

pub type OmniDirectionalLightsStaticBuffer = RenderHandle<UniformGPUData<OmniDirectionalLight>>;
pub type DirectionalLightsStaticBuffer = RenderHandle<UniformGPUData<DirectionalLight>>;
pub type SpotLightsStaticBuffer = RenderHandle<UniformGPUData<SpotLight>>;

macro_rules! impl_static_buffer_accessor {
    ($name:ident, $buffer_type:ty, $type:ty) => {
        paste::paste! {
            pub fn [<acquire_ $name>](&mut self) -> $buffer_type {
                self.$name.transfer()
            }
            pub fn [<release_ $name>](&mut self, $name: $buffer_type) {
                self.$name = $name;
            }
            pub fn [<$name _structured_buffer_view>](&self) -> BufferView{
                self.$name.structured_buffer_view($type::SIZE as u64)
            }
        }
    };
}

unsafe impl Send for Renderer {}

unsafe impl Sync for Renderer {}

impl Renderer {
    pub fn new() -> Result<Self> {
        #![allow(unsafe_code)]
        let num_render_frames = 2usize;
        let api = unsafe { GfxApi::new(&ApiDef::default()).unwrap() };
        let device_context = api.device_context();
        let filesystem = FileSystem::new(".")?;
        filesystem.add_mount_point("renderer", env!("CARGO_MANIFEST_DIR"))?;

        let shader_compiler = HlslCompiler::new(filesystem).unwrap();

        cgen::initialize(device_context);

        let static_buffer = UnifiedStaticBuffer::new(device_context, 64 * 1024 * 1024, false);
        let test_transform_data = TransformStaticsBuffer::new(UniformGPUData::<
            cgen::cgen_type::EntityTransforms,
        >::new(
            &static_buffer, 64 * 1024
        ));

        let omnidirectional_lights_data =
            OmniDirectionalLightsStaticBuffer::new(UniformGPUData::<OmniDirectionalLight>::new(
                &static_buffer,
                OmniDirectionalLight::PAGE_SIZE,
            ));

        let directional_lights_data =
            DirectionalLightsStaticBuffer::new(UniformGPUData::<DirectionalLight>::new(
                &static_buffer,
                DirectionalLight::PAGE_SIZE,
            ));

        let spotlights_data = SpotLightsStaticBuffer::new(UniformGPUData::<SpotLight>::new(
            &static_buffer,
            SpotLight::PAGE_SIZE,
        ));

        let descriptor_heap_def = DescriptorHeapDef {
            max_descriptor_sets: 32 * 4096,
            sampler_count: 32 * 128,
            constant_buffer_count: 32 * 1024,
            buffer_count: 32 * 1024,
            rw_buffer_count: 32 * 1024,
            texture_count: 32 * 1024,
            rw_texture_count: 32 * 1024,
        };

        Ok(Self {
            frame_idx: 0,
            render_frame_idx: 0,
            num_render_frames,
            prev_frame_sems: (0..num_render_frames)
                .map(|_| device_context.create_semaphore())
                .collect(),
            sparse_unbind_sems: (0..num_render_frames)
                .map(|_| device_context.create_semaphore())
                .collect(),
            sparse_bind_sems: (0..num_render_frames)
                .map(|_| device_context.create_semaphore())
                .collect(),
            frame_fences: (0..num_render_frames)
                .map(|_| device_context.create_fence().unwrap())
                .collect(),
            graphics_queue: RwLock::new(device_context.create_queue(QueueType::Graphics).unwrap()),
            descriptor_heap: device_context
                .create_descriptor_heap(&descriptor_heap_def)
                .unwrap(),
            command_buffer_pools: Mutex::new(GpuSafePool::new(num_render_frames)),
            descriptor_pools: Mutex::new(GpuSafePool::new(num_render_frames)),
            transient_buffer: TransientPagedBuffer::new(device_context, 512, 64 * 1024),
            static_buffer,
            transforms_data: test_transform_data,
            omnidirectional_lights_data,
            directional_lights_data,
            spotlights_data,
            bump_allocator_pool: Mutex::new(CpuPool::new()),
            shader_compiler,
            api,
        })
    }

    pub fn device_context(&self) -> &DeviceContext {
        self.api.device_context()
    }

    pub fn num_render_frames(&self) -> usize {
        self.num_render_frames
    }

    pub fn render_frame_idx(&self) -> usize {
        self.render_frame_idx
    }

    pub fn graphics_queue_guard(&self, queue_type: QueueType) -> RwLockReadGuard<'_, Queue> {
        match queue_type {
            QueueType::Graphics => self.graphics_queue.read(),
            _ => unreachable!(),
        }
    }

    pub fn shader_compiler(&self) -> HlslCompiler {
        self.shader_compiler.clone()
    }

    // TMP: change that.
    pub(crate) fn transient_buffer(&self) -> TransientPagedBuffer {
        self.transient_buffer.clone()
    }

    pub fn acquire_transform_data(&mut self) -> TransformStaticsBuffer {
        self.transforms_data.transfer()
    }

    pub fn release_transform_data(&mut self, test: TransformStaticsBuffer) {
        self.transforms_data = test;
    }

    impl_static_buffer_accessor!(
        omnidirectional_lights_data,
        OmniDirectionalLightsStaticBuffer,
        OmniDirectionalLight
    );

    impl_static_buffer_accessor!(
        directional_lights_data,
        DirectionalLightsStaticBuffer,
        DirectionalLight
    );

    impl_static_buffer_accessor!(spotlights_data, SpotLightsStaticBuffer, SpotLight);

    pub fn static_buffer(&self) -> &UnifiedStaticBuffer {
        &self.static_buffer
    }

    pub fn test_add_update_jobs(&self, job_blocks: &mut Vec<UniformGPUDataUploadJobBlock>) {
        self.static_buffer.add_update_job_block(job_blocks);
    }

    #[span_fn]
    pub fn flush_update_jobs(&self, render_context: &RenderContext<'_>) {
        let prev_frame_semaphore = &self.prev_frame_sems[self.render_frame_idx];
        let unbind_semaphore = &self.sparse_unbind_sems[self.render_frame_idx];
        let bind_semaphore = &self.sparse_bind_sems[self.render_frame_idx];

        self.static_buffer.flush_updater(
            prev_frame_semaphore,
            unbind_semaphore,
            bind_semaphore,
            render_context,
        );
    }

    pub fn static_buffer_ro_view(&self) -> BufferView {
        self.static_buffer.read_only_view()
    }

    //    pub fn prev_frame_semaphore(&self)

    pub(crate) fn acquire_command_buffer_pool(
        &self,
        queue_type: QueueType,
    ) -> CommandBufferPoolHandle {
        let queue = self.graphics_queue_guard(queue_type);
        let mut pool = self.command_buffer_pools.lock();
        pool.acquire_or_create(|| CommandBufferPool::new(&*queue))
    }

    pub(crate) fn release_command_buffer_pool(&self, handle: CommandBufferPoolHandle) {
        let mut pool = self.command_buffer_pools.lock();
        pool.release(handle);
    }

    pub(crate) fn acquire_descriptor_pool(
        &self,
        heap_def: &DescriptorHeapDef,
    ) -> DescriptorPoolHandle {
        let mut pool = self.descriptor_pools.lock();
        pool.acquire_or_create(|| DescriptorPool::new(self.descriptor_heap.clone(), heap_def))
    }

    pub(crate) fn release_descriptor_pool(&self, handle: DescriptorPoolHandle) {
        let mut pool = self.descriptor_pools.lock();
        pool.release(handle);
    }

    pub(crate) fn acquire_bump_allocator(&self) -> BumpAllocatorHandle {
        let mut pool = self.bump_allocator_pool.lock();
        pool.acquire_or_create(BumpAllocator::new)
    }

    pub(crate) fn release_bump_allocator(&self, handle: BumpAllocatorHandle) {
        let mut pool = self.bump_allocator_pool.lock();
        pool.release(handle);
    }

    #[span_fn]
    pub(crate) fn begin_frame(&mut self) {
        //
        // Update frame indices
        //
        self.frame_idx += 1;
        self.render_frame_idx = self.frame_idx % self.num_render_frames;

        //
        // Wait for the next cpu frame to be available
        //
        let signal_fence = &self.frame_fences[self.render_frame_idx];
        if signal_fence.get_fence_status().unwrap() == FenceStatus::Incomplete {
            signal_fence.wait().unwrap();
        }

        //
        // Now, it is safe to free memory
        //
        let device_context = self.api.device_context();
        device_context.free_gpu_memory();

        //
        // Broadcast begin frame event
        //
        {
            let mut pool = self.command_buffer_pools.lock();
            pool.begin_frame();
        }
        {
            let mut pool = self.descriptor_pools.lock();
            pool.begin_frame();
        }
        {
            let mut pool = self.bump_allocator_pool.lock();
            pool.begin_frame();
        }

        // TMP: todo
        self.transient_buffer.begin_frame();
    }

    #[span_fn]
    pub(crate) fn end_frame(&mut self) {
        let graphics_queue = self.graphics_queue.write();
        let frame_fence = &self.frame_fences[self.render_frame_idx];

        graphics_queue
            .submit(&[], &[], &[], Some(frame_fence))
            .unwrap();

        //
        // Broadcast end frame event
        //

        {
            let mut pool = self.command_buffer_pools.lock();
            pool.end_frame();
        }
        {
            let mut pool = self.descriptor_pools.lock();
            pool.end_frame();
        }
        {
            let mut pool = self.bump_allocator_pool.lock();
            pool.end_frame();
        }
    }

    #[span_fn]
    pub(crate) fn prepare_vs_ps(&self, shader_source: String) -> Shader {
        let device_context = self.device_context();

        let shader_compiler = self.shader_compiler();
        let shader_build_result = shader_compiler
            .compile(&CompileParams {
                shader_source: ShaderSource::Path(shader_source),
                global_defines: Vec::new(),
                entry_points: vec![
                    EntryPoint {
                        defines: Vec::new(),
                        name: "main_vs".to_owned(),
                        target_profile: TargetProfile::Vertex,
                    },
                    EntryPoint {
                        defines: Vec::new(),
                        name: "main_ps".to_owned(),
                        target_profile: TargetProfile::Pixel,
                    },
                ],
            })
            .unwrap();

        let vert_shader_module = device_context
            .create_shader_module(
                ShaderPackage::SpirV(shader_build_result.spirv_binaries[0].bytecode.clone())
                    .module_def(),
            )
            .unwrap();

        let frag_shader_module = device_context
            .create_shader_module(
                ShaderPackage::SpirV(shader_build_result.spirv_binaries[1].bytecode.clone())
                    .module_def(),
            )
            .unwrap();

        device_context.create_shader(vec![
            ShaderStageDef {
                entry_point: "main_vs".to_owned(),
                shader_stage: ShaderStageFlags::VERTEX,
                shader_module: vert_shader_module,
            },
            ShaderStageDef {
                entry_point: "main_ps".to_owned(),
                shader_stage: ShaderStageFlags::FRAGMENT,
                shader_module: frag_shader_module,
            },
        ])
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        {
            let graphics_queue = self.graphics_queue_guard(QueueType::Graphics);
            graphics_queue.wait_for_queue_idle().unwrap();
        }
        std::mem::drop(self.spotlights_data.take());
        std::mem::drop(self.directional_lights_data.take());
        std::mem::drop(self.omnidirectional_lights_data.take());
        std::mem::drop(self.transforms_data.take());

        cgen::shutdown();
    }
}