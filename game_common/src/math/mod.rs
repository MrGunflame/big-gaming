mod rotation;

use glam::Vec3;
pub use rotation::{Rotation, RotationExt};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn point(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    pub fn plane_intersection(&self, plane_origin: Vec3, plane_normal: Vec3) -> Option<Vec3> {
        let denom = plane_normal.dot(self.direction);
        if denom.abs() > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(plane_normal) / denom;
            Some(self.origin + self.direction * distance)
        } else {
            None
        }
    }
}
