use bevy::prelude::{Camera3d, Entity, Query, ResMut, Transform, With};
use bevy_egui::egui::Window;
use bevy_egui::EguiContext;

use crate::components::Rotation;
use crate::entities::player::PlayerCharacter;

pub fn debug(
    mut egui: ResMut<EguiContext>,
    entities: Query<Entity>,
    players: Query<(&Transform, &Rotation), With<PlayerCharacter>>,
    cameras: Query<(&Transform, &Rotation), With<Camera3d>>,
) {
    let (player, rotation) = players.single();
    let (camera, camera_rot) = cameras.single();

    Window::new("Debug").show(egui.ctx_mut(), |ui| {
        ui.label(format!("Entity count: {}", entities.into_iter().count()));

        let x = player.translation.x;
        let y = player.translation.y;
        let z = player.translation.z;

        let yaw = rotation.yaw();
        let pitch = rotation.pitch();

        ui.label(format!("Player at: X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));
        ui.label(format!("Looking at: Yaw: {} Pitch: {}", yaw, pitch));
    });
}
