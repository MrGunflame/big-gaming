use glam::Vec3;

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
