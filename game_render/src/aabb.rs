use glam::Vec3;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl Aabb {
    pub fn min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.half_extents
    }

    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        let center = (min + max) * 0.5;
        let half_extents = (max - min) * 0.5;

        Self {
            center,
            half_extents,
        }
    }
}
