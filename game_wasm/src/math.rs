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
}

impl Real for f32 {
    #[inline]
    fn abs(self) -> Self {
        f32::abs(self)
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
}

impl Real for f64 {
    #[inline]
    fn abs(self) -> Self {
        f64::abs(self)
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
}

impl private::Sealed for f32 {}
impl private::Sealed for f64 {}

mod private {
    pub trait Sealed {}
}
