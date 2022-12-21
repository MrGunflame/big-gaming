use bevy::prelude::{Camera3d, Transform, With};
use bevy_egui::egui::Window;
use bevy_rapier3d::prelude::Velocity;
use common::world::chunk::ChunkId;

use crate::components::Rotation;
use crate::entities::player::PlayerCharacter;

use super::Interface;

#[derive(Default)]
pub struct Debug {}

impl Interface for Debug {
    fn kind(&self) -> super::InterfaceKind {
        super::InterfaceKind::Widget
    }

    fn create(&mut self) {}

    fn render(&mut self, ctx: &bevy_egui::egui::Context, world: &mut bevy::prelude::World) {
        let entities = world.entities().len();

        Window::new("Debug").resizable(true).show(ctx, |ui| {
            ui.label(format!("Entity count: {}", entities));

            let (player, rotation, velocity) = world
                .query_filtered::<(&Transform, &Rotation, &Velocity), With<PlayerCharacter>>()
                .single(world);

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

            let (camera, camera_rot) = world
                .query_filtered::<(&Transform, &Rotation), With<Camera3d>>()
                .single(world);

            let x = camera.translation.x;
            let y = camera.translation.y;
            let z = camera.translation.z;
            let yaw = camera_rot.yaw();
            let pitch = camera_rot.pitch();

            ui.label(format!("Camera at X: {:.2} Y: {:.2} Z: {:.2}", x, y, z));
            ui.label(format!("Looking at: Yaw: {} Pitch: {}", yaw, pitch));
        });
    }

    fn destroy(&mut self) {}
}
