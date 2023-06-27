use std::f32::consts::PI;
use std::fmt::{self, Display, Formatter};

use game_common::math::RotationExt;
use glam::{Quat, Vec3};

pub fn extract_actor_rotation(rotation: Quat) -> Quat {
    let mut pt = rotation.dir_vec();

    if pt.y == 1.0 {
        return rotation;
    }

    pt.y = 0.0;
    if !pt.is_normalized() {
        pt = pt.normalize();
    }

    let b = Vec3::Z;

    let mut angle = (pt.dot(b)).acos();
    if pt.x.is_sign_negative() {
        angle = -angle;
    }

    Quat::from_axis_angle(Vec3::Y, angle + PI)
}

/// An angle in degrees.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl Degrees {
    #[inline]
    pub const fn new(val: f32) -> Self {
        Self(val)
    }

    pub fn to_radians(self) -> Radians {
        Radians(self.0.to_radians())
    }

    pub fn to_f32(self) -> f32 {
        self.0
    }

    pub fn sin(self) -> f32 {
        self.to_radians().sin()
    }

    pub fn cos(self) -> f32 {
        self.to_radians().cos()
    }
}

impl Display for Degrees {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}°", self.0)
    }
}

/// An angle in radians.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Radians(pub f32);

impl Radians {
    pub const fn new(val: f32) -> Self {
        Self(val)
    }

    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0.to_degrees())
    }

    pub fn to_f32(self) -> f32 {
        self.0
    }

    pub fn sin(self) -> f32 {
        self.0.sin()
    }

    pub fn cos(self) -> f32 {
        self.0.cos()
    }
}

impl Display for Radians {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl From<Degrees> for Radians {
    fn from(deg: Degrees) -> Self {
        deg.to_radians()
    }
}

impl From<Radians> for Degrees {
    fn from(rad: Radians) -> Self {
        rad.to_degrees()
    }
}
