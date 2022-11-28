use bevy::prelude::{Component, EulerRot, Mat3, Quat, Vec2, Vec3};

use crate::utils::Radians;

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct Rotation {
    yaw: f32,
    pitch: f32,
}

impl Rotation {
    pub const fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub const fn yaw(self) -> Radians {
        Radians(self.yaw)
    }

    pub const fn pitch(self) -> Radians {
        Radians(self.pitch)
    }

    pub fn with_yaw<T>(mut self, yaw: T) -> Self
    where
        T: Into<Radians>,
    {
        self.yaw = yaw.into().to_f32();
        self
    }

    pub fn with_pitch<T>(mut self, pitch: T) -> Self
    where
        T: Into<Radians>,
    {
        self.pitch = pitch.into().to_f32();
        self
    }

    pub fn add_yaw<T>(mut self, yaw: T) -> Self
    where
        T: Into<Radians>,
    {
        self.yaw += yaw.into().to_f32();
        self
    }

    pub fn add_pitch<T>(mut self, pitch: T) -> Self
    where
        T: Into<Radians>,
    {
        self.pitch += pitch.into().to_f32();
        self
    }

    /// Returns a new `Rotation` rotated the the left.
    pub fn left<T>(self, rot: T) -> Self
    where
        T: Into<Radians>,
    {
        self.with_yaw(Radians(self.yaw().to_f32() + rot.into().to_f32()))
    }

    /// Returns a new `Rotation` rotated to the right.
    pub fn right<T>(self, rot: T) -> Self
    where
        T: Into<Radians>,
    {
        self.with_yaw(Radians(-self.yaw().to_f32() + rot.into().to_f32()))
    }

    pub fn to_mat3(self) -> Mat3 {
        Mat3::from_euler(EulerRot::YXZ, -self.yaw, -self.pitch, 0.0)
    }

    pub fn to_quat(self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, -self.yaw, -self.pitch, 0.0)
    }

    /// Calculates a movement vector with this `Rotation`.
    ///
    /// Foward: -Z
    /// Up: Y
    /// Right: X
    pub fn movement_vec(self) -> Vec3 {
        // We don't want the entity to go flying, so disregard pitch here.
        let x = self.yaw.sin();
        let y = 0.0;
        let z = -self.yaw.cos();

        Vec3 { x, y, z }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    use crate::utils::{Degrees, Radians};

    use super::Rotation;

    #[test]
    fn test_rotation_left() {
        let mut rot = Rotation::new();
        assert_eq!(rot.yaw, 0.0);
        assert_eq!(rot.pitch, 0.0);

        rot = rot.left(Radians(PI));
        assert_eq!(rot.yaw, PI);

        rot = rot.left(Degrees(90.0));
        assert_eq!(rot.yaw, PI * 1.5);
    }

    #[test]
    fn test_rotation_movement_vec() {
        let rot = Rotation::new();
        let vec = rot.movement_vec();

        assert_eq!(vec.x, 0.0);
        assert_eq!(vec.y, 0.0);
        assert_eq!(vec.z, -1.0);

        let rot = Rotation::new().with_yaw(Radians(FRAC_PI_2));
        let vec = rot.movement_vec();

        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 0.0);
        // assert!(vec.z >= 0.0 && vec.z < 0.000001);
        assert_eq!(vec.z, 0.0);

        let rot = Rotation::new().with_yaw(Radians(FRAC_PI_4));
        let vec = rot.movement_vec();
        assert_eq!(vec.x, 0.70710677);
        assert_eq!(vec.y, 0.0);
        assert_eq!(vec.z, -0.70710677);
    }
}
