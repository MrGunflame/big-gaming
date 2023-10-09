//! 3D Transform editing
//!

use game_common::components::transform::Transform;
use game_common::math::Ray;
use game_core::hierarchy::Key;
use glam::{Quat, Vec2, Vec3};

use super::Axis;

#[derive(Clone, Debug, Default)]
pub struct EditOperation {
    /// The cursor translation when the edit started.
    origin: Vec2,
    camera_ray: Ray,
    nodes: Vec<EditNode>,
    mode: EditMode,
    camera_rotation: Quat,
}

impl EditOperation {
    pub const fn new() -> Self {
        Self {
            origin: Vec2::ZERO,
            nodes: Vec::new(),
            mode: EditMode::None,
            camera_ray: Ray::ZERO,
            camera_rotation: Quat::IDENTITY,
        }
    }

    pub fn create(&mut self, cursor_origin: Vec2, camera_ray: Ray, camera_rotation: Quat) {
        self.origin = cursor_origin;
        self.camera_ray = camera_ray;
        self.camera_rotation = camera_rotation;
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
            EditMode::Rotate(axis) => {
                for node in &mut self.nodes {
                    let p1 = self
                        .camera_ray
                        .plane_intersection(
                            node.current.translation,
                            self.camera_rotation * Vec3::Z,
                        )
                        .unwrap();
                    let p2 = ray
                        .plane_intersection(node.current.translation, camera_rotation * Vec3::Z)
                        .unwrap();

                    let a1 = (p1 - node.current.translation).normalize_or_zero();
                    let a2 = (p2 - node.current.translation).normalize_or_zero();

                    dbg!(p1, p2, a1, a2);

                    let angle = a1.dot(a2).clamp(-1.0, 1.0).acos();
                    dbg!(angle);
                    let rotation = Quat::from_axis_angle(Vec3::Y, angle);

                    node.current.rotation = (node.origin.rotation * rotation).normalize();
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
