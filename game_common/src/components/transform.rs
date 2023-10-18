use std::ops::{Deref, DerefMut, Mul, MulAssign};

use glam::{Mat3, Mat4, Quat, Vec3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::splat(0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(1.0),
    };

    pub const fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    pub const fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    pub const fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

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

    pub fn compute_matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn mul_transform(self, transform: Transform) -> Self {
        if cfg!(debug_assertions) {
            assert_transform(self);
            assert_transform(transform);
        }

        let translation = self.transform_point(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn transform_point(self, mut point: Vec3) -> Vec3 {
        point = self.scale * point;
        point = self.rotation * point;
        point += self.translation;
        point
    }

    pub fn is_valid(self) -> bool {
        self.translation.is_finite()
            && self.rotation.is_finite()
            && self.rotation.is_normalized()
            && self.scale.is_finite()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// The [`Transform`] of a component at the previous frame.
///
/// This can be used to calculate delta movement. It should only be added to components than can
/// move.
#[derive(Copy, Clone, Debug, Default)]
pub struct PreviousTransform(pub Transform);

impl Deref for PreviousTransform {
    type Target = Transform;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PreviousTransform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Mul for Transform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.mul_transform(rhs)
    }
}

impl MulAssign for Transform {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct GlobalTransform(pub Transform);

#[track_caller]
fn assert_transform(transform: Transform) {
    assert!(
        transform.translation.is_finite(),
        "invalid translation value: {:?}",
        transform.translation,
    );
    assert!(
        transform.translation.is_finite() && transform.rotation.is_normalized(),
        "invalid rotation value: {:?}",
        transform.rotation,
    );
    assert!(
        transform.scale.is_finite(),
        "invalid scale value: {:?}",
        transform.scale,
    );
}
