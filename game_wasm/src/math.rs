pub use glam::{Quat, Vec2, Vec3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

pub trait Real: private::Sealed {
    fn abs(self) -> Self;

    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;

    fn asin(self) -> Self;
    fn acos(self) -> Self;
    fn atan(self) -> Self;

    fn sinh(self) -> Self;
    fn cosh(self) -> Self;
    fn tanh(self) -> Self;

    fn asinh(self) -> Self;
    fn acosh(self) -> Self;
    fn atanh(self) -> Self;

    fn floor(self) -> Self;
    fn ceil(self) -> Self;

    fn sqrt(self) -> Self;
    fn cbrt(self) -> Self;

    fn copysign(self, sign: Self) -> Self;
    fn signum(self) -> Self;
}

impl Real for f32 {
    #[inline]
    fn abs(self) -> Self {
        if self.is_sign_negative() {
            -self
        } else {
            self
        }
    }

    #[inline]
    fn sin(self) -> Self {
        libm::sinf(self)
    }

    #[inline]
    fn cos(self) -> Self {
        libm::cosf(self)
    }

    #[inline]
    fn tan(self) -> Self {
        libm::tanf(self)
    }

    #[inline]
    fn asin(self) -> Self {
        libm::sinf(self)
    }

    #[inline]
    fn acos(self) -> Self {
        libm::acosf(self)
    }

    #[inline]
    fn atan(self) -> Self {
        libm::atanf(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        libm::ceilf(self)
    }

    #[inline]
    fn floor(self) -> Self {
        libm::floorf(self)
    }

    #[inline]
    fn acosh(self) -> Self {
        libm::acoshf(self)
    }

    #[inline]
    fn asinh(self) -> Self {
        libm::asinhf(self)
    }

    #[inline]
    fn atanh(self) -> Self {
        libm::atanhf(self)
    }

    #[inline]
    fn sqrt(self) -> Self {
        libm::sqrtf(self)
    }

    #[inline]
    fn cbrt(self) -> Self {
        libm::cbrtf(self)
    }

    #[inline]
    fn sinh(self) -> Self {
        libm::sinhf(self)
    }

    #[inline]
    fn cosh(self) -> Self {
        libm::coshf(self)
    }

    #[inline]
    fn tanh(self) -> Self {
        libm::tanhf(self)
    }

    #[inline]
    fn copysign(self, sign: Self) -> Self {
        libm::copysignf(self, sign)
    }

    #[inline]
    fn signum(self) -> Self {
        if self.is_nan() {
            Self::NAN
        } else {
            1.0.copysign(self)
        }
    }
}

impl Real for f64 {
    #[inline]
    fn abs(self) -> Self {
        if self.is_sign_negative() {
            -self
        } else {
            self
        }
    }

    #[inline]
    fn acos(self) -> Self {
        libm::acos(self)
    }

    #[inline]
    fn acosh(self) -> Self {
        libm::acosh(self)
    }

    #[inline]
    fn asin(self) -> Self {
        libm::asin(self)
    }

    #[inline]
    fn asinh(self) -> Self {
        libm::asinh(self)
    }

    #[inline]
    fn atan(self) -> Self {
        libm::atan(self)
    }

    #[inline]
    fn atanh(self) -> Self {
        libm::atanh(self)
    }

    #[inline]
    fn cbrt(self) -> Self {
        libm::cbrt(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        libm::ceil(self)
    }

    #[inline]
    fn cos(self) -> Self {
        libm::cos(self)
    }

    #[inline]
    fn cosh(self) -> Self {
        libm::cosh(self)
    }

    #[inline]
    fn floor(self) -> Self {
        libm::floor(self)
    }

    #[inline]
    fn sin(self) -> Self {
        libm::sin(self)
    }

    #[inline]
    fn sinh(self) -> Self {
        libm::sinh(self)
    }

    #[inline]
    fn sqrt(self) -> Self {
        libm::sqrt(self)
    }

    #[inline]
    fn tan(self) -> Self {
        libm::tan(self)
    }

    #[inline]
    fn tanh(self) -> Self {
        libm::tanh(self)
    }

    #[inline]
    fn copysign(self, sign: Self) -> Self {
        libm::copysign(self, sign)
    }

    #[inline]
    fn signum(self) -> Self {
        if self.is_nan() {
            Self::NAN
        } else {
            1.0.copysign(self)
        }
    }
}

impl private::Sealed for f32 {}
impl private::Sealed for f64 {}

/// Extension trait for objects representing an 3D rotation.
pub trait RotationExt: private::Sealed {
    /// The orientation representing facing to the front.
    ///
    /// Equivalent to the [`IDENTITY`] of the rotation.
    ///
    /// [`IDENTITY`]: Quat::IDENTITY
    const FORWARD: Self;

    /// The orientation representing facing to the back.
    const BACKWARD: Self;

    /// The orientation representing facing to the left.
    const LEFT: Self;

    /// The orientation representing facing to the right.
    const RIGHT: Self;

    /// The orientation representing facing up.
    const UP: Self;

    /// THe orientation representing facing down.
    const DOWN: Self;
}

impl RotationExt for Quat {
    // Note: We can't use float math in const items so we need to
    // precompute all values.

    const FORWARD: Self = Quat::IDENTITY;

    // Quat::from_axis_angle(Vec3::Y, PI)
    const BACKWARD: Self = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);

    // Quat::from_axis_angle(Vec3::Y, PI / 2)
    const RIGHT: Self = Quat::from_xyzw(0.0, 0.7071067811865475, 0.0, 0.7071067811865475);

    // Quat::from_axis_angle(Vec3::Y, -PI / 2)
    const LEFT: Self = Quat::from_xyzw(0.0, -0.7071067811865475, 0.0, 0.7071067811865475);

    // Quat::from_axis_angle(Vec3::X, PI / 2)
    const UP: Self = Quat::from_xyzw(0.7071067811865475, 0.0, 0.0, 0.7071067811865475);

    // Quat::from_axis_angle(Vec3::X, -PI / 2)
    const DOWN: Self = Quat::from_xyzw(-0.7071067811865475, 0.0, 0.0, 0.7071067811865475);
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
        assert_eq!(Quat::FORWARD, Quat::IDENTITY);
        assert_eq!(Quat::BACKWARD, Quat::from_axis_angle(Vec3::Y, PI));
        assert_eq!(Quat::RIGHT, Quat::from_axis_angle(Vec3::Y, FRAC_PI_2));
        assert_eq!(Quat::LEFT, Quat::from_axis_angle(Vec3::Y, -FRAC_PI_2));
        assert_eq!(Quat::UP, Quat::from_axis_angle(Vec3::X, FRAC_PI_2));
        assert_eq!(Quat::DOWN, Quat::from_axis_angle(Vec3::X, -FRAC_PI_2));
    }
}
