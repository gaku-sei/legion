// This is generated file. Do not edit manually

#![allow(clippy::all)]
#![allow(dead_code)]

use lgn_graphics_api::DeviceContext;
pub mod cgen_type;
pub mod descriptor_set;
pub mod pipeline_layout;

pub fn initialize(device_context: &DeviceContext) {
    descriptor_set::FrameDescriptorSet::initialize(device_context);
    descriptor_set::ViewDescriptorSet::initialize(device_context);
    descriptor_set::EguiDescriptorSet::initialize(device_context);
    descriptor_set::PickingDescriptorSet::initialize(device_context);

    let descriptor_set_layouts = [
        descriptor_set::FrameDescriptorSet::descriptor_set_layout(),
        descriptor_set::ViewDescriptorSet::descriptor_set_layout(),
        descriptor_set::EguiDescriptorSet::descriptor_set_layout(),
        descriptor_set::PickingDescriptorSet::descriptor_set_layout(),
    ];

    pipeline_layout::EguiPipelineLayout::initialize(device_context, &descriptor_set_layouts);
    pipeline_layout::ConstColorPipelineLayout::initialize(device_context, &descriptor_set_layouts);
    pipeline_layout::PickingPipelineLayout::initialize(device_context, &descriptor_set_layouts);
    pipeline_layout::ShaderPipelineLayout::initialize(device_context, &descriptor_set_layouts);
}

pub fn shutdown() {
    descriptor_set::FrameDescriptorSet::shutdown();
    descriptor_set::ViewDescriptorSet::shutdown();
    descriptor_set::EguiDescriptorSet::shutdown();
    descriptor_set::PickingDescriptorSet::shutdown();

    pipeline_layout::EguiPipelineLayout::shutdown();
    pipeline_layout::ConstColorPipelineLayout::shutdown();
    pipeline_layout::PickingPipelineLayout::shutdown();
    pipeline_layout::ShaderPipelineLayout::shutdown();
}