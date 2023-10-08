//! 3D Transform editing
//!

use game_common::components::transform::Transform;
use game_common::math::Ray;
use game_core::hierarchy::Key;
use glam::{Quat, Vec2, Vec3};

use super::Axis;

#[derive(Clone, Debug, Default)]
pub struct EditOperation {
    origin: Vec2,
    nodes: Vec<EditNode>,
    mode: EditMode,
}

impl EditOperation {
    pub const fn new() -> Self {
        Self {
            origin: Vec2::ZERO,
            nodes: Vec::new(),
            mode: EditMode::None,
        }
    }

    pub fn create(&mut self, cursor_origin: Vec2) {
        self.origin = cursor_origin;
    }

    pub fn push(&mut self, id: Key, origin: Transform) {
        self.nodes.push(EditNode {
            id,
            origin,
            current: origin,
        });
    }

    pub fn mode(&self) -> EditMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: EditMode) {
        if self.mode != mode {
            self.reset_to_origin();
            self.mode = mode;
        }
    }

    pub fn update(
        &mut self,
        ray: Ray,
        camera_rotation: Quat,
    ) -> impl Iterator<Item = (Key, Transform)> + '_ {
        match self.mode {
            EditMode::Translate(axis) => {
                for node in &mut self.nodes {
                    // Find the intersection of the camera ray with the plane placed
                    // at the object, facing the camera. The projected point is the new
                    // translation.
                    let plane_origin = node.current.translation;
                    let plane_normal = camera_rotation * Vec3::Z;
                    // FIXME: What if no intersection?
                    let point = ray.plane_intersection(plane_origin, plane_normal).unwrap();

                    match axis {
                        Some(Axis::X) => node.current.translation.x = point.x,
                        Some(Axis::Y) => node.current.translation.y = point.y,
                        Some(Axis::Z) => node.current.translation.z = point.z,
                        None => node.current.translation = point,
                    }
                }
            }
            EditMode::None => (),
            _ => todo!(),
        }

        self.nodes.iter().map(|node| (node.id, node.current))
    }

    pub fn confirm(&mut self) {
        self.nodes.clear();
        self.mode = EditMode::None;
    }

    pub fn reset(&mut self) -> impl Iterator<Item = (Key, Transform)> + '_ {
        self.nodes.drain(..).map(|node| (node.id, node.origin))
    }

    fn reset_to_origin(&mut self) {
        for node in &mut self.nodes {
            node.current = node.origin;
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum EditMode {
    #[default]
    None,
    Translate(Option<Axis>),
    Rotate(Option<Axis>),
    Scale(Option<Axis>),
}

/// A node being edited.
#[derive(Copy, Clone, Debug)]
struct EditNode {
    id: Key,
    /// The origin of the node before the edit started.
    origin: Transform,
    /// The current transform.
    current: Transform,
}
