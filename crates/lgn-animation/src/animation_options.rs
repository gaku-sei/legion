use lgn_ecs::prelude::{Res, ResMut};

use lgn_graphics_renderer::egui::Egui;

#[derive(Default)]
pub(crate) struct AnimationOptions {
    pub(crate) show_collision_geometry: bool,
}

pub(crate) fn ui_animation_options(
    egui: Res<'_, Egui>,
    mut animation_options: ResMut<'_, AnimationOptions>,
) {
    egui.window("Animation options", |ui| {
        ui.checkbox(
            &mut animation_options.show_collision_geometry,
            "Show animation bones",
        );
    });

    drop(egui);
}
