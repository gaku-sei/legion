use crate::{
    animation_options::AnimationOptions, components::GraphDefinition,
    runtime_graph::node_state_machine::StateInfo,
};
use lgn_core::BumpAllocatorPool;
use lgn_ecs::prelude::{Query, Res};
use lgn_graphics_data::Color;
use lgn_graphics_renderer::{debug_display::DebugDisplay, resources::DefaultMeshType};

pub(crate) fn display_animation_2(
    debug_display: Res<'_, DebugDisplay>,
    bump_allocator_pool: Res<'_, BumpAllocatorPool>,
    animation_options: Res<'_, AnimationOptions>,
    mut graphs: Query<'_, '_, &mut GraphDefinition2>,
) {
    if !animation_options.show_animation_skeleton_bones {
        return;
    }

    bump_allocator_pool.scoped_bump(|bump| {
        debug_display.create_display_list(bump, |builder| {
            for mut graph in graphs.iter_mut() {
                let current_node_index = graph.current_node_index;

                let active_state: &mut StateInfo = (*graph.nodes[current_node_index as usize])
                    .get_active_state()
                    .unwrap();

                let clip = (*active_state.state_node.child_node).get_clip().unwrap();

                for n_bone in 0..clip.poses[clip.current_key_frame_index as usize]
                    .skeleton
                    .bone_ids
                    .len()
                {
                    // let current_clip = &graph.nodes[graph.current_node_index as usize].clip;
                    let bone_depth: u8 = clip.poses[clip.current_key_frame_index as usize]
                        .skeleton
                        .get_bone_depth(
                            clip.poses[clip.current_key_frame_index as usize]
                                .skeleton
                                .bone_ids[n_bone]
                                .unwrap(),
                        )
                        .try_into()
                        .unwrap();
                    let color_interval: u8 = (255
                        / clip.poses[clip.current_key_frame_index as usize]
                            .skeleton
                            .get_max_bone_depth()
                            .unwrap())
                    .try_into()
                    .unwrap();
                    let debug_color = Color::new(bone_depth * color_interval, 255, 52, 255);
                    builder.add_default_mesh(
                        &clip.poses[clip.current_key_frame_index as usize].transforms[n_bone]
                            .global,
                        DefaultMeshType::Sphere,
                        debug_color,
                    );
                }
            }
        });
    });
    drop(debug_display);
    drop(bump_allocator_pool);
    drop(animation_options);
    drop(graphs);
}
