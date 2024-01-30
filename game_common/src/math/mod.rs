use glam::{Quat, Vec3};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub const ZERO: Self = Self {
        origin: Vec3::ZERO,
        direction: Vec3::ZERO,
    };

    pub fn point(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// Returns the intersection point of the `Ray` and a plane. Returns `None` if they never
    /// intersect.
    pub fn plane_intersection(&self, plane_origin: Vec3, plane_normal: Vec3) -> Option<Vec3> {
        let denom = plane_normal.dot(self.direction);
        if denom.abs() > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(plane_normal) / denom;
            Some(self.origin + self.direction * distance)
        } else {
            None
        }
    }

    /// Returns the intersection point of the `Ray` and a plane under the assumption that the `Ray`
    /// always intersects the plane.
    ///
    /// This is exactly the case when the ray is parallel to the plane, i.e. when `self.direction`
    /// is orthogonal to `plane_normal`.
    ///
    /// If the `Ray` might not intersect the plane consider using [`plane_intersection`].
    ///
    /// [`plane_intersection`]: Self::plane_intersection
    pub fn plane_intersection_unchecked(&self, plane_origin: Vec3, plane_normal: Vec3) -> Vec3 {
        let denom = plane_normal.dot(self.direction);
        let distance = (plane_origin - self.origin).dot(plane_normal) / denom;
        self.origin + self.direction * distance
    }
}

/// Extension trait for objects representing an 3D rotation.
pub trait RotationExt: private::Sealed {
    /// The orientation representing the front.
    ///
    /// Equivalent to the [`IDENTITY`] of the rotation.
    ///
    /// [`IDENTITY`]: Quat::IDENTITY
    const FRONT: Self;

    /// The orientation representing the back.
    const BACK: Self;

    /// The orientation representing the left.
    const LEFT: Self;

    /// The orientation representing the right.
    const RIGHT: Self;

    /// The orientation representing the top.
    const TOP: Self;

    /// THe orientation representing the bottom.
    const BOTTOM: Self;
}

impl RotationExt for Quat {
    // Note: We can't use float math in const items so we need to
    // precompute all values.

    const FRONT: Self = Quat::IDENTITY;

    // Quat::from_axis_angle(Vec3::Y, PI)
    const BACK: Self = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);

    // Quat::from_axis_angle(Vec3::Y, PI / 2)
    const RIGHT: Self = Quat::from_xyzw(0.0, 0.7071067811865475, 0.0, 0.7071067811865475);

    // Quat::from_axis_angle(Vec3::Y, -PI / 2)
    const LEFT: Self = Quat::from_xyzw(0.0, -0.7071067811865475, 0.0, 0.7071067811865475);

    // Quat::from_axis_angle(Vec3::X, PI / 2)
    const TOP: Self = Quat::from_xyzw(0.7071067811865475, 0.0, 0.0, 0.7071067811865475);

    // Quat::from_axis_angle(Vec3::X, -PI / 2)
    const BOTTOM: Self = Quat::from_xyzw(-0.7071067811865475, 0.0, 0.0, 0.7071067811865475);
}

impl private::Sealed for Quat {}

mod private {
    pub trait Sealed {}
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use glam::{Quat, Vec3};

    use crate::math::RotationExt;

    #[test]
    fn rotation_ext_consts() {
        assert_eq!(Quat::FRONT, Quat::IDENTITY);
        assert_eq!(Quat::BACK, Quat::from_axis_angle(Vec3::Y, PI));
        assert_eq!(Quat::RIGHT, Quat::from_axis_angle(Vec3::Y, FRAC_PI_2));
        assert_eq!(Quat::LEFT, Quat::from_axis_angle(Vec3::Y, -FRAC_PI_2));
        assert_eq!(Quat::TOP, Quat::from_axis_angle(Vec3::X, FRAC_PI_2));
        assert_eq!(Quat::BOTTOM, Quat::from_axis_angle(Vec3::X, -FRAC_PI_2));
    }
}
