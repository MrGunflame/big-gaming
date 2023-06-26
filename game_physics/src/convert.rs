use glam::{Quat, Vec3};
use nalgebra::{Quaternion, UnitQuaternion};
use rapier3d::prelude::{Point, Real, Rotation, Vector};

pub fn vector(v: Vec3) -> Vector<Real> {
    Vector::new(v.x, v.y, v.z)
}

pub fn rotation(v: Quat) -> Rotation<Real> {
    UnitQuaternion::new_normalize(Quaternion::new(v.w, v.x, v.y, v.z))
}

pub fn point(v: Vec3) -> Point<Real> {
    Point::new(v.x, v.y, v.z)
}

pub fn vec3(v: Vector<Real>) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub fn quat(v: Rotation<Real>) -> Quat {
    Quat::from_xyzw(v.i, v.j, v.k, v.w)
}
