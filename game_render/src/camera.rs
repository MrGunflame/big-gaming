use std::f32::consts::PI;

use bevy_ecs::prelude::{Component, Entity, EventReader};
use bevy_ecs::system::Query;
use game_window::events::WindowResized;
use glam::{Mat3, Mat4, Quat, Vec3};

#[derive(Clone, Debug, Component)]
pub struct Camera {
    pub target: RenderTarget,
}

#[derive(Clone, Debug)]
pub enum RenderTarget {
    /// Render to a window surface.
    Window(Entity),
    Image(),
}

/// Perspective camera projection paramters
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Projection {
    pub aspect_ratio: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            aspect_ratio: 1.0,
            fov: PI / 4.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

pub const OPENGL_TO_WGPU: Mat4 = Mat4::from_cols_array_2d(&[
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.5, 0.0],
    [0.0, 0.0, 0.5, 1.0],
]);

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn looking_at(self, target: Vec3, up: Vec3) -> Self {
        self.looking_to(target - self.translation, up)
    }

    pub fn looking_to(mut self, direction: Vec3, up: Vec3) -> Self {
        let forward = -direction.normalize();
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
        self
    }
}

pub fn update_camera_aspect_ratio(
    mut cameras: Query<&mut Camera>,
    mut events: EventReader<WindowResized>,
) {
    for event in events.iter() {
        // let camera = cameras.get();

        event.width as f32 / event.height as f32;
    }
}
