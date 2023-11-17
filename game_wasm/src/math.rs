pub use glam::{Quat, Vec2, Vec3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

pub trait Real {
    fn acos(self) -> Self;
}

impl Real for f32 {
    #[inline]
    fn acos(self) -> Self {
        libm::acosf(self)
    }
}
