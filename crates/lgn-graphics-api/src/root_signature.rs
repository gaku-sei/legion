use crate::backends::BackendRootSignature;
use crate::deferred_drop::Drc;
use crate::{DescriptorSetLayout, DeviceContext, GfxResult};

// Not currently exposed
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DynamicDescriptorIndex(pub(crate) u32);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PushConstantIndex(pub(crate) u32);

#[derive(Debug, Clone, Copy, Hash)]
pub struct PushConstantDef {
    pub size: u32,
}

#[derive(Default, Debug)]
pub struct RootSignatureDef {
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
    pub push_constant_def: Option<PushConstantDef>,
}

impl Clone for RootSignatureDef {
    fn clone(&self) -> Self {
        Self {
            descriptor_set_layouts: self.descriptor_set_layouts.clone(),
            push_constant_def: self.push_constant_def,
        }
    }
}

#[derive(Debug)]
pub(crate) struct RootSignatureInner {
    device_context: DeviceContext,
    definition: RootSignatureDef,
    pub(crate) backend_root_signature: BackendRootSignature,
}

impl Drop for RootSignatureInner {
    fn drop(&mut self) {
        self.backend_root_signature.destroy(&self.device_context);
    }
}

#[derive(Debug, Clone)]
pub struct RootSignature {
    pub(crate) inner: Drc<RootSignatureInner>,
}

impl RootSignature {
    pub fn new(device_context: &DeviceContext, definition: &RootSignatureDef) -> GfxResult<Self> {
        let backend_root_signature = BackendRootSignature::new(device_context, definition)?;

        let inner = RootSignatureInner {
            device_context: device_context.clone(),
            definition: definition.clone(),
            backend_root_signature,
        };

        Ok(Self {
            inner: device_context.deferred_dropper().new_drc(inner),
        })
    }

    pub fn device_context(&self) -> &DeviceContext {
        &self.inner.device_context
    }

    pub fn definition(&self) -> &RootSignatureDef {
        &self.inner.definition
    }
}