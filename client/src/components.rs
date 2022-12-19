use std::f32::consts::PI;
use std::num::NonZeroU32;

use bevy::prelude::{Component, EulerRot, Mat3, Quat, Transform, Vec3};

use crate::utils::Radians;

/// A entity that exists within the game world.
///
/// This only includes that exist within the world, i.e. excludes components like cameras, markers,
/// etc...
#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct WorldObject;

/// A temporary [`WorldObject`] that does not consist between runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct TemporaryObject;

/// A permanent [`WorldObject`] that should be resumed between runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct PersistentObject;

/// An object that can be interacted with.
#[derive(Clone, Debug)]
pub struct Interactable {
    pub name: Option<String>,
}

/// A entitiy that can act within a world.
///
/// Unlike static props, `Actor` is a marker component for all entities that may act on its own,
/// or be acted upon. This most notably includes player characters and NPCs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct Actor;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct ActorState(NonZeroU32);

impl ActorState {
    pub const NORMAL: Self = Self(NonZeroU32::new(1).unwrap());
    pub const DEAD: Self = Self(NonZeroU32::new(2).unwrap());
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct Player;

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

    /// Add pitch to the `Rotation`, saturating at the min/max ranges of [`-(PI/2)`:`PI/2`] (excl).
    pub fn saturating_add_pitch<T>(mut self, pitch: T) -> Self
    where
        T: Into<Radians>,
    {
        self = self.add_pitch(pitch);

        if self.pitch < -(PI / 2.0) {
            self.pitch = -(PI / 2.0);
        } else if self.pitch > PI / 2.0 {
            self.pitch = PI / 2.0;
        };

        self
    }

    /// Returns a new `Rotation` rotated the the left.
    pub fn left<T>(self, rot: T) -> Self
    where
        T: Into<Radians>,
    {
        self.with_yaw(Radians(self.yaw().to_f32() - rot.into().to_f32()))
    }

    /// Returns a new `Rotation` rotated to the right.
    pub fn right<T>(self, rot: T) -> Self
    where
        T: Into<Radians>,
    {
        self.with_yaw(Radians(self.yaw().to_f32() + rot.into().to_f32()))
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
        let x = self.yaw.sin() * self.pitch.cos();
        let y = -self.pitch.sin();
        let z = -self.yaw.cos() * self.pitch.cos();

        Vec3 { x, y, z }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PreviousTransform(pub Transform);

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
        // FP inaccuracy
        assert!(vec.z >= 0.0 && vec.z < 0.000001);

        let rot = Rotation::new().with_yaw(Radians(FRAC_PI_4));
        let vec = rot.movement_vec();
        assert_eq!(vec.x, 0.70710677);
        assert_eq!(vec.y, 0.0);
        assert_eq!(vec.z, -0.70710677);
    }
}
