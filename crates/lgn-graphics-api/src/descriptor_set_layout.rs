use std::sync::atomic::Ordering;

use crate::{
    backends::BackendDescriptorSetLayout, deferred_drop::Drc, DeviceContext, GfxResult,
    ShaderResourceType, MAX_DESCRIPTOR_BINDINGS,
};

static NEXT_DESCRIPTOR_SET_LAYOUT_ID: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(1);

#[derive(Clone, Debug)]
pub struct Descriptor {
    pub name: String,
    pub binding: u32,
    pub shader_resource_type: ShaderResourceType,
    pub element_count: u32,
    pub update_data_offset: u32,
}

impl Descriptor {
    pub fn element_count_normalized(&self) -> u32 {
        self.element_count.max(1)
    }
}

#[derive(Debug, Clone, Hash)]
pub struct DescriptorDef {
    pub name: String,
    pub binding: u32,
    pub shader_resource_type: ShaderResourceType,
    pub array_size: u32,
}

impl DescriptorDef {
    pub fn array_size_normalized(&self) -> u32 {
        self.array_size.max(1u32)
    }
}

#[derive(Debug, Clone, Hash)]
pub struct DescriptorSetLayoutDef {
    pub frequency: u32,
    pub descriptor_defs: Vec<DescriptorDef>,
}

impl DescriptorSetLayoutDef {
    pub fn new() -> Self {
        Self {
            frequency: 0,
            descriptor_defs: Vec::new(),
        }
    }
}

impl Default for DescriptorSetLayoutDef {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug)]
pub(crate) struct DescriptorSetLayoutInner {
    device_context: DeviceContext,
    definition: DescriptorSetLayoutDef,
    id: u32,
    frequency: u32,
    binding_mask: u64,
    descriptors: Vec<Descriptor>,

    pub(crate) backend_layout: BackendDescriptorSetLayout,
}

impl Drop for DescriptorSetLayoutInner {
    fn drop(&mut self) {
        self.backend_layout.destroy(&self.device_context);
    }
}

#[derive(Debug, Clone)]
pub struct DescriptorSetLayout {
    pub(crate) inner: Drc<DescriptorSetLayoutInner>,
}

impl DescriptorSetLayout {
    pub fn device_context(&self) -> &DeviceContext {
        &self.inner.device_context
    }

    pub fn definition(&self) -> &DescriptorSetLayoutDef {
        &self.inner.definition
    }

    pub fn uid(&self) -> u32 {
        self.inner.id
    }

    pub fn frequency(&self) -> u32 {
        self.inner.frequency
    }

    pub fn binding_mask(&self) -> u64 {
        self.inner.binding_mask
    }

    pub fn find_descriptor_index_by_name(&self, name: &str) -> Option<usize> {
        self.inner
            .descriptors
            .iter()
            .position(|descriptor| name == descriptor.name)
    }

    pub fn descriptor(&self, index: usize) -> &Descriptor {
        &self.inner.descriptors[index]
    }

    pub fn new(
        device_context: &DeviceContext,
        definition: &DescriptorSetLayoutDef,
    ) -> GfxResult<Self> {
        let mut binding_mask = 0;
        for descriptor_def in &definition.descriptor_defs {
            assert!((descriptor_def.binding as usize) < MAX_DESCRIPTOR_BINDINGS);
            let mask = 1u64 << descriptor_def.binding;
            assert!((binding_mask & mask) == 0, "Binding already in use");
            binding_mask |= mask;
        }
        let (backend_layout, descriptors) =
            BackendDescriptorSetLayout::new(device_context, definition)?;

        let descriptor_set_layout_id =
            NEXT_DESCRIPTOR_SET_LAYOUT_ID.fetch_add(1, Ordering::Relaxed);

        let result = Self {
            inner: device_context
                .deferred_dropper()
                .new_drc(DescriptorSetLayoutInner {
                    device_context: device_context.clone(),
                    definition: definition.clone(),
                    id: descriptor_set_layout_id,
                    frequency: definition.frequency,
                    binding_mask,
                    descriptors,
                    backend_layout,
                }),
        };

        Ok(result)
    }
}

impl PartialEq for DescriptorSetLayout {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}