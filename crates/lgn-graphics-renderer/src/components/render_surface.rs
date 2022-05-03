use std::{cmp::max, sync::Arc};

use lgn_ecs::prelude::Component;
use lgn_graphics_api::{
    DepthStencilClearValue, DepthStencilRenderTargetBinding, DeviceContext, Extents2D, Format,
    GPUViewType, LoadOp, ResourceState, ResourceUsage, Semaphore, SemaphoreDef, StoreOp,
};
use lgn_window::WindowId;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use crate::egui::egui_pass::EguiPass;
use crate::gpu_renderer::HzbSurface;
use crate::hl_gfx_api::HLCommandBuffer;
use crate::render_pass::{
    DebugRenderPass, FinalResolveRenderPass, PickingRenderPass, RenderTarget,
};
use crate::resources::PipelineManager;
use crate::{RenderContext, Renderer};

pub trait Presenter: Send + Sync {
    fn resize(&mut self, device_context: &DeviceContext, extents: RenderSurfaceExtents);
    fn present(&mut self, render_context: &RenderContext<'_>, render_surface: &mut RenderSurface);
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderSurfaceId(Uuid);

impl RenderSurfaceId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSurfaceExtents {
    extents: Extents2D,
}

impl RenderSurfaceExtents {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            extents: Extents2D {
                width: max(1u32, width),
                height: max(1u32, height),
            },
        }
    }

    pub fn width(self) -> u32 {
        self.extents.width
    }

    pub fn height(self) -> u32 {
        self.extents.height
    }
}

pub struct RenderSurfaces {
    window_id_mapper: HashMap<WindowId, RenderSurfaceId>,
}

impl RenderSurfaces {
    pub fn new() -> Self {
        Self {
            window_id_mapper: HashMap::new(),
        }
    }

    pub fn insert(&mut self, window_id: WindowId, render_surface_id: RenderSurfaceId) {
        let result = self.window_id_mapper.insert(window_id, render_surface_id);
        assert!(result.is_none());
    }

    pub fn remove(&mut self, window_id: WindowId) {
        let result = self.window_id_mapper.remove(&window_id);
        assert!(result.is_some());
    }

    pub fn get_from_window_id(&self, window_id: WindowId) -> Option<&RenderSurfaceId> {
        self.window_id_mapper.get(&window_id)
    }
}

/// An event that is sent whenever a render surface is created for a window
#[derive(Debug, Clone)]
pub struct RenderSurfaceCreatedForWindow {
    pub window_id: WindowId,
    pub render_surface_id: RenderSurfaceId,
}

#[allow(dead_code)]
struct SizeDependentResources {
    hdr_rt: RenderTarget,
    depth_rt: RenderTarget,
    hzb_surface: HzbSurface,
    hzb_init: bool,
}

impl SizeDependentResources {
    fn new(
        device_context: &DeviceContext,
        extents: RenderSurfaceExtents,
        pipeline_manager: &PipelineManager,
    ) -> Self {
        Self {
            hdr_rt: RenderTarget::new(
                device_context,
                "HDR_RT",
                extents,
                Format::R16G16B16A16_SFLOAT,
                ResourceUsage::AS_RENDER_TARGET
                    | ResourceUsage::AS_SHADER_RESOURCE
                    | ResourceUsage::AS_TRANSFERABLE,
                GPUViewType::RenderTarget,
            ),
            depth_rt: RenderTarget::new(
                device_context,
                "Depth_RT",
                extents,
                Format::D32_SFLOAT,
                ResourceUsage::AS_DEPTH_STENCIL | ResourceUsage::AS_SHADER_RESOURCE,
                GPUViewType::DepthStencil,
            ),
            hzb_surface: HzbSurface::new(device_context, extents, pipeline_manager),
            hzb_init: false,
        }
    }
}

pub enum RenderSurfacePresentingStatus {
    Presenting,
    Paused,
}

#[derive(Component)]
pub struct RenderSurface {
    id: RenderSurfaceId,
    extents: RenderSurfaceExtents,
    resources: SizeDependentResources,
    presenters: Vec<Box<dyn Presenter>>,
    // tmp
    num_render_frames: u64,
    render_frame_idx: u64,
    presenter_sems: Vec<Semaphore>,
    picking_renderpass: Arc<RwLock<PickingRenderPass>>,
    debug_renderpass: Arc<RwLock<DebugRenderPass>>,
    egui_renderpass: Arc<RwLock<EguiPass>>,
    final_resolve_render_pass: Arc<RwLock<FinalResolveRenderPass>>,
    presenting_status: RenderSurfacePresentingStatus,
}

impl RenderSurface {
    pub fn new(
        renderer: &Renderer,
        pipeline_manager: &PipelineManager,
        extents: RenderSurfaceExtents,
    ) -> Self {
        Self::new_with_id(RenderSurfaceId::new(), renderer, pipeline_manager, extents)
    }

    pub fn extents(&self) -> RenderSurfaceExtents {
        self.extents
    }

    pub fn picking_renderpass(&self) -> Arc<RwLock<PickingRenderPass>> {
        self.picking_renderpass.clone()
    }

    pub fn debug_renderpass(&self) -> Arc<RwLock<DebugRenderPass>> {
        self.debug_renderpass.clone()
    }

    pub fn egui_renderpass(&self) -> Arc<RwLock<EguiPass>> {
        self.egui_renderpass.clone()
    }

    pub fn final_resolve_render_pass(&self) -> Arc<RwLock<FinalResolveRenderPass>> {
        self.final_resolve_render_pass.clone()
    }

    pub fn resize(
        &mut self,
        device_context: &DeviceContext,
        extents: RenderSurfaceExtents,
        pipeline_manager: &PipelineManager,
    ) {
        if self.extents != extents {
            self.resources = SizeDependentResources::new(device_context, extents, pipeline_manager);
            for presenter in &mut self.presenters {
                presenter.resize(device_context, extents);
            }
            self.extents = extents;
        }
    }

    pub fn register_presenter<T: 'static + Presenter>(&mut self, create_fn: impl FnOnce() -> T) {
        let presenter = create_fn();
        self.presenters.push(Box::new(presenter));
    }

    pub fn id(&self) -> RenderSurfaceId {
        self.id
    }

    pub fn hdr_rt(&self) -> &RenderTarget {
        &self.resources.hdr_rt
    }

    pub fn hdr_rt_mut(&mut self) -> &mut RenderTarget {
        &mut self.resources.hdr_rt
    }

    pub fn depth_rt(&self) -> &RenderTarget {
        &self.resources.depth_rt
    }

    pub fn depth_rt_mut(&mut self) -> &mut RenderTarget {
        &mut self.resources.depth_rt
    }

    pub(crate) fn init_hzb_if_needed(
        &mut self,
        render_context: &RenderContext<'_>,
        cmd_buffer: &mut HLCommandBuffer<'_>,
    ) {
        if !self.resources.hzb_init {
            self.resources
                .depth_rt
                .transition_to(cmd_buffer, ResourceState::DEPTH_WRITE);

            cmd_buffer.begin_render_pass(
                &[],
                &Some(DepthStencilRenderTargetBinding {
                    texture_view: self.resources.depth_rt.rtv(),
                    depth_load_op: LoadOp::Clear,
                    stencil_load_op: LoadOp::DontCare,
                    depth_store_op: StoreOp::Store,
                    stencil_store_op: StoreOp::DontCare,
                    clear_value: DepthStencilClearValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                }),
            );
            cmd_buffer.end_render_pass();

            self.generate_hzb(render_context, cmd_buffer);

            self.resources.hzb_init = true;
        }
    }

    pub(crate) fn generate_hzb(
        &mut self,
        render_context: &RenderContext<'_>,
        cmd_buffer: &mut HLCommandBuffer<'_>,
    ) {
        cmd_buffer.with_label("Generate HZB", |cmd_buffer| {
            self.depth_rt_mut()
                .transition_to(cmd_buffer, ResourceState::PIXEL_SHADER_RESOURCE);

            self.get_hzb_surface()
                .generate_hzb(render_context, cmd_buffer, self.depth_rt().srv());

            self.depth_rt_mut()
                .transition_to(cmd_buffer, ResourceState::DEPTH_WRITE);
        });
    }

    pub(crate) fn get_hzb_surface(&self) -> &HzbSurface {
        &self.resources.hzb_surface
    }

    /// Call the `present` method of all the registered presenters.
    /// No op if the render surface is "paused", i.e., it's `presenting`
    /// attribute is `false`.
    pub fn present(&mut self, render_context: &RenderContext<'_>) {
        if matches!(
            self.presenting_status,
            RenderSurfacePresentingStatus::Paused
        ) {
            return;
        }

        let mut presenters = std::mem::take(&mut self.presenters);

        for presenter in &mut presenters {
            presenter.as_mut().present(render_context, self);
        }

        self.presenters = presenters;
    }

    //
    // TODO: change that asap. Acquire can't be called more than once per frame.
    // This would result in a crash.
    //
    pub fn acquire(&mut self) -> &Semaphore {
        let render_frame_idx = (self.render_frame_idx + 1) % self.num_render_frames;
        let presenter_sem = &self.presenter_sems[render_frame_idx as usize];
        self.render_frame_idx = render_frame_idx;

        presenter_sem
    }

    pub fn presenter_sem(&self) -> &Semaphore {
        &self.presenter_sems[self.render_frame_idx as usize]
    }

    pub fn pause(&mut self) -> &mut Self {
        self.presenting_status = RenderSurfacePresentingStatus::Paused;
        self
    }

    pub fn resume(&mut self) -> &mut Self {
        self.presenting_status = RenderSurfacePresentingStatus::Presenting;
        self
    }

    fn new_with_id(
        id: RenderSurfaceId,
        renderer: &Renderer,
        pipeline_manager: &PipelineManager,
        extents: RenderSurfaceExtents,
    ) -> Self {
        let num_render_frames = renderer.num_render_frames();
        let device_context = renderer.device_context();
        let presenter_sems = (0..num_render_frames)
            .map(|_| device_context.create_semaphore(SemaphoreDef::default()))
            .collect();
        Self {
            id,
            extents,
            resources: SizeDependentResources::new(device_context, extents, pipeline_manager),
            num_render_frames,
            render_frame_idx: 0,
            presenter_sems,
            picking_renderpass: Arc::new(RwLock::new(PickingRenderPass::new(device_context))),
            debug_renderpass: Arc::new(RwLock::new(DebugRenderPass::new(pipeline_manager))),
            egui_renderpass: Arc::new(RwLock::new(EguiPass::new(device_context, pipeline_manager))),
            final_resolve_render_pass: Arc::new(RwLock::new(FinalResolveRenderPass::new(
                device_context,
                pipeline_manager,
            ))),
            presenters: Vec::new(),
            presenting_status: RenderSurfacePresentingStatus::Presenting,
        }
    }
}
