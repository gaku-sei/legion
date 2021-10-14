use std::num::NonZeroU32;

use fnv::FnvHashMap;

use crate::{
    DescriptorDef, DescriptorSetLayoutDef, DeviceContext, GfxApi, GfxResult, PipelineType,
    PushConstant, PushConstantDef, RootSignatureDef, Shader, ShaderResource, ShaderStageFlags,
    MAX_DESCRIPTOR_SET_LAYOUTS,
};

pub(crate) static NEXT_TEXTURE_ID: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(1);

pub(crate) fn extract_resources<A: GfxApi>(
    shaders: &[A::Shader],
) -> GfxResult<(PipelineType, Vec<ShaderResource>, Option<PushConstant>)> {
    let mut merged_resources: Vec<ShaderResource> = vec![];
    let mut merged_resources_name_index_map = FnvHashMap::default();
    let mut pipeline_type = None;
    let mut push_constant = None;

    // Make sure all shaders are compatible/build lookup of shared data from them
    for shader in shaders {
        log::trace!(
            "Merging resources from shader with reflection info: {:?}",
            shader.pipeline_reflection()
        );
        let pipeline_reflection = shader.pipeline_reflection();

        let shader_pipeline_type = if pipeline_reflection
            .shader_stages
            .intersects(ShaderStageFlags::COMPUTE)
        {
            PipelineType::Compute
        } else {
            PipelineType::Graphics
        };

        if pipeline_type.is_none() {
            pipeline_type = Some(shader_pipeline_type);
        } else if pipeline_type != Some(shader_pipeline_type) {
            log::error!("Shaders with different pipeline types are sharing a root signature");
            return Err(
                "Shaders with different pipeline types are sharing a root signature".into(),
            );
        }

        for resource in &pipeline_reflection.shader_resources {
            log::trace!(
                "  Merge resource (set={:?} binding={:?} name={:?})",
                resource.set_index,
                resource.binding,
                resource.name
            );

            let existing_resource_index = merged_resources_name_index_map.get(&resource.name);

            if let Some(&existing_resource_index) = existing_resource_index {
                log::trace!("    Resource with this name already exists");
                //
                // This binding name already exists, make sure they match up. Then merge
                // the shader stage flags.
                //
                let existing_resource: &mut ShaderResource =
                    &mut merged_resources[existing_resource_index];
                if existing_resource.set_index != resource.set_index {
                    let message = format!(
                            "Shader resource (set={:?} binding={:?} name={:?}) has mismatching set {:?} and {:?} across shaders in same root signature",
                            resource.set_index,
                            resource.binding,
                            resource.name,
                            resource.set_index,
                            existing_resource.set_index
                        );
                    log::error!("{}", message);
                    return Err(message.into());
                }

                if existing_resource.binding != resource.binding {
                    let message = format!(
                            "Shader resource (set={:?} binding={:?} name={:?}) has mismatching binding {:?} and {:?} across shaders in same root signature",
                            resource.set_index,
                            resource.binding,
                            resource.name,
                            resource.binding,
                            existing_resource.binding
                        );
                    log::error!("{}", message);
                    return Err(message.into());
                }

                verify_resources_can_overlap(resource, existing_resource)?;

                // for previous_resource in &mut resources {
                //     if previous_resource.name == resource.name {
                //         previous_resource.used_in_shader_stages |= resource.used_in_shader_stages;
                //     }
                // }

                existing_resource.used_in_shader_stages |= resource.used_in_shader_stages;
            } else {
                //
                // We have not seen a resource by this name yet or the name is not set. See if
                // it overlaps an existing binding that doesn't share the same name.
                //
                let mut existing_index = None;
                for (index, x) in merged_resources.iter().enumerate() {
                    if x.used_in_shader_stages
                        .intersects(resource.used_in_shader_stages)
                        && x.binding == resource.binding
                        && x.set_index == resource.set_index
                    {
                        existing_index = Some(index);
                    }
                }

                if let Some(existing_index) = existing_index {
                    log::trace!("    No resource by this name exists yet, checking if it overlaps with a previous resource");

                    //
                    // It's a new binding name that overlaps an existing binding. Check that
                    // they are compatible types. If they are, alias them.
                    //
                    let existing_resource = &mut merged_resources[existing_index];
                    verify_resources_can_overlap(resource, existing_resource)?;

                    let old =
                        merged_resources_name_index_map.insert(&resource.name, existing_index);
                    assert!(old.is_none());

                    log::trace!(
                        "Adding shader flags {:?} the existing resource",
                        resource.used_in_shader_stages
                    );
                    existing_resource.used_in_shader_stages |= resource.used_in_shader_stages;
                } else {
                    //
                    // It's a new binding name and doesn't overlap with existing bindings
                    //
                    log::trace!("    Does not collide with existing bindings");
                    merged_resources_name_index_map.insert(&resource.name, merged_resources.len());
                    merged_resources.push(resource.clone());
                }
            }
        }

        match &mut push_constant {
            None => {
                push_constant = pipeline_reflection.push_constant;
            }
            Some(pc) => {
                if let Some(pipeline_push_constant) = &pipeline_reflection.push_constant {
                    pc.verify_compatible_across_stages(pipeline_push_constant)?;
                    pc.used_in_shader_stages |= pipeline_push_constant.used_in_shader_stages;
                }
            }
        }
    }

    Ok((pipeline_type.unwrap(), merged_resources, push_constant))
}

fn verify_resources_can_overlap(
    resource: &ShaderResource,
    previous_resource: &ShaderResource,
) -> GfxResult<()> {
    if previous_resource.element_count_normalized() != resource.element_count_normalized() {
        let message = format!(
            "Shader resource (set={:?} binding={:?} name={:?}) has mismatching element_count {:?} and {:?} across shaders in same root signature",
            resource.set_index,
            resource.binding,
            resource.name,
            resource.element_count_normalized(),
            previous_resource.element_count_normalized()
        );
        log::error!("{}", message);
        return Err(message.into());
    }

    if previous_resource.shader_resource_type != resource.shader_resource_type {
        let message = format!(
            "Shader resource (set={:?} binding={:?} name={:?}) has mismatching resource_type {:?} and {:?} across shaders in same root signature",
            resource.set_index,
            resource.binding,
            resource.name,
            resource.shader_resource_type,
            previous_resource.shader_resource_type
        );
        log::error!("{}", message);
        return Err(message.into());
    }

    Ok(())
}

pub fn tmp_extract_root_signature_def<A: GfxApi>(
    device_context: &A::DeviceContext,
    shaders: &[A::Shader],
) -> GfxResult<RootSignatureDef<A>> {
    let (pipeline_type, shader_resources, push_constant) = extract_resources::<A>(shaders)?;

    for shader_resource in &shader_resources {
        shader_resource.validate()?;
    }

    let mut layouts: [Option<A::DescriptorSetLayout>; MAX_DESCRIPTOR_SET_LAYOUTS] =
        [None, None, None, None];

    for set_index in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
        let mut set_resources = shader_resources
            .iter()
            .filter(|sr| sr.set_index as usize == set_index)
            .collect::<Vec<_>>();

        if !set_resources.is_empty() {
            set_resources.sort_by(|a, b| a.binding.cmp(&b.binding));

            let mut layout_def = DescriptorSetLayoutDef::new();

            let mut add_descriptor = |d: &ShaderResource| {
                let descriptor_def = DescriptorDef {
                    name: d.name.clone(),
                    binding: d.binding,
                    shader_resource_type: d.shader_resource_type,
                    array_size: d.element_count,
                };
                layout_def.descriptor_defs.push(descriptor_def);
            };

            set_resources.iter().for_each(|x| add_descriptor(x));

            layouts[set_index as usize] =
                Some(device_context.create_descriptorset_layout(&layout_def)?);
        }
    }

    let push_constant_def = push_constant.map(|e| PushConstantDef {
        used_in_shader_stages: e.used_in_shader_stages,
        size: NonZeroU32::new(e.size).unwrap(),
    });

    Ok(RootSignatureDef {
        pipeline_type,
        descriptor_set_layouts: layouts,
        push_constant_def,
    })
}
