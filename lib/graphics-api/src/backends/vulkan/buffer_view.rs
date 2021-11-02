use crate::backends::deferred_drop::Drc;
use crate::{Buffer, BufferView, BufferViewDef, GPUViewType, GfxResult, ShaderResourceType};

use super::{VulkanApi, VulkanBuffer, VulkanDescriptor};

#[derive(Clone, Debug)]
struct VulkanBufferViewInner {
    view_def: BufferViewDef,
    buffer: VulkanBuffer,
    vk_offset: u64,
    vk_size: u64,
}

#[derive(Clone, Debug)]
pub struct VulkanBufferView {
    inner: Drc<VulkanBufferViewInner>,
}

impl VulkanBufferView {
    pub fn from_buffer(buffer: &VulkanBuffer, view_def: &BufferViewDef) -> GfxResult<Self> {
        view_def.verify(buffer.buffer_def());

        let device_context = buffer.device_context();
        let vk_offset = view_def.byte_offset;
        let vk_size = view_def.element_size * view_def.element_count;

        Ok(Self {
            inner: device_context
                .deferred_dropper()
                .new_drc(VulkanBufferViewInner {
                    view_def: *view_def,
                    buffer: buffer.clone(),
                    vk_offset,
                    vk_size,
                }),
        })
    }

    pub(super) fn vk_offset(&self) -> u64 {
        self.inner.vk_offset
    }

    pub(super) fn vk_size(&self) -> u64 {
        self.inner.vk_size
    }

    pub(super) fn is_compatible_with_descriptor(&self, descriptor: &VulkanDescriptor) -> bool {
        match descriptor.shader_resource_type {
            ShaderResourceType::ConstantBuffer => {
                self.inner.view_def.gpu_view_type == GPUViewType::ConstantBufferView
            }
            ShaderResourceType::StructuredBuffer | ShaderResourceType::ByteAdressBuffer => {
                self.inner.view_def.gpu_view_type == GPUViewType::ShaderResourceView
            }
            ShaderResourceType::RWStructuredBuffer | ShaderResourceType::RWByteAdressBuffer => {
                self.inner.view_def.gpu_view_type == GPUViewType::UnorderedAccessView
            }
            // ShaderResourceType::Undefined |
            ShaderResourceType::Sampler
            | ShaderResourceType::Texture2D
            | ShaderResourceType::RWTexture2D
            | ShaderResourceType::Texture2DArray
            | ShaderResourceType::RWTexture2DArray
            | ShaderResourceType::Texture3D
            | ShaderResourceType::RWTexture3D
            | ShaderResourceType::TextureCube
            | ShaderResourceType::TextureCubeArray => false,
        }
    }
}

impl BufferView<VulkanApi> for VulkanBufferView {
    fn view_def(&self) -> &BufferViewDef {
        &self.inner.view_def
    }

    fn buffer(&self) -> &VulkanBuffer {
        &self.inner.buffer
    }
}
