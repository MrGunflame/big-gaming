use bevy::prelude::{Camera3d, Entity, Query, Res, ResMut, Transform, With};
use bevy_egui::egui::Window;
use bevy_egui::EguiContext;
use bevy_rapier3d::prelude::Velocity;
use common::world::chunk::ChunkId;

use crate::components::Rotation;
use crate::entities::player::PlayerCharacter;

use super::interfaces::MENU_DEBUG;
use super::InterfaceState;

pub fn debug(
    mut egui: ResMut<EguiContext>,
    entities: Query<Entity>,
    state: Res<InterfaceState>,
    players: Query<(&Transform, &Rotation, &Velocity), With<PlayerCharacter>>,
    cameras: Query<(&Transform, &Rotation), With<Camera3d>>,
) {
    if !state.contains(MENU_DEBUG) {
        return;
    }

    let (player, rotation, velocity) = players.single();
    let (camera, camera_rot) = cameras.single();

    Window::new("Debug")
        .resizable(true)
        .show(egui.ctx_mut(), |ui| {
            ui.label(format!("Entity count: {}", entities.into_iter().count()));

            let x = player.translation.x;
            let y = player.translation.y;
            let z = player.translation.z;

            let yaw = rotation.yaw();
            let pitch = rotation.pitch();

            ui.label(format!("Player at: X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));
            ui.label(format!("Looking at: Yaw: {} Pitch: {}", yaw, pitch));
            ui.label(format!("Chunk {:?}", ChunkId::new(x, y, z).as_parts()));

            let linvel = velocity.linvel;
            let angvel = velocity.angvel;

            ui.label(format!(
                "Linvel: {:.2} {:.2} {:.2}",
                linvel.x, linvel.y, linvel.y
            ));
            ui.label(format!(
                "Angvel: {:.2} {:.2} {:.2}",
                angvel.x, angvel.y, angvel.z
            ));

            let x = camera.translation.x;
            let y = camera.translation.y;
            let z = camera.translation.z;
            let yaw = camera_rot.yaw();
            let pitch = camera_rot.pitch();

            ui.label(format!("Camera at X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));
            ui.label(format!("Looking at: Yaw: {} Pitch: {}", yaw, pitch));
        });
}
