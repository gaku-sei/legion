use super::{VulkanApi, VulkanDescriptorSetLayout, VulkanDeviceContext};
use crate::{GfxResult, PipelineType, RootSignature, RootSignatureDef, MAX_DESCRIPTOR_SET_LAYOUTS};

use ash::vk;
use std::sync::Arc;

// Not currently exposed
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DynamicDescriptorIndex(pub(crate) u32);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PushConstantIndex(pub(crate) u32);

#[derive(Debug)]
struct RootSignatureVulkanInner {
    device_context: VulkanDeviceContext,
    pipeline_type: PipelineType,
    layouts: [Option<VulkanDescriptorSetLayout>; MAX_DESCRIPTOR_SET_LAYOUTS],
    pipeline_layout: vk::PipelineLayout,
}

impl Drop for RootSignatureVulkanInner {
    fn drop(&mut self) {
        let device = self.device_context.device();

        unsafe {
            device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

#[derive(Clone, Debug)]
pub struct VulkanRootSignature {
    inner: Arc<RootSignatureVulkanInner>,
}

impl VulkanRootSignature {
    pub fn device_context(&self) -> &VulkanDeviceContext {
        &self.inner.device_context
    }

    pub fn vk_pipeline_layout(&self) -> vk::PipelineLayout {
        self.inner.pipeline_layout
    }

    pub fn new(
        device_context: &VulkanDeviceContext,
        root_signature_def: &RootSignatureDef<VulkanApi>,
    ) -> GfxResult<Self> {
        log::trace!("Create VulkanRootSignature");

        //
        // Create pipeline layout
        //
        let mut vk_descriptor_set_layouts =
            [vk::DescriptorSetLayout::null(); MAX_DESCRIPTOR_SET_LAYOUTS];

        let mut descriptor_set_layout_count = 0;
        for layout in root_signature_def
            .descriptor_set_layouts
            .iter()
            .filter_map(|x| x.as_ref())
        {
            vk_descriptor_set_layouts[descriptor_set_layout_count] = layout.vk_layout();
            descriptor_set_layout_count += 1;
        }

        let mut push_constant_ranges = Vec::new();
        if let Some(push_constant_def) = &root_signature_def.push_constant_def {
            push_constant_ranges.push(vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::ALL,
                offset: 0,
                size: push_constant_def.size.get(),
            });
        }

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&vk_descriptor_set_layouts[0..descriptor_set_layout_count])
            .push_constant_ranges(&push_constant_ranges)
            .build();

        let pipeline_layout = unsafe {
            device_context
                .device()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        let inner = RootSignatureVulkanInner {
            device_context: device_context.clone(),
            pipeline_type: root_signature_def.pipeline_type,
            layouts: root_signature_def.descriptor_set_layouts.clone(),
            pipeline_layout,
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

impl RootSignature<VulkanApi> for VulkanRootSignature {
    fn pipeline_type(&self) -> PipelineType {
        self.inner.pipeline_type
    }
}
