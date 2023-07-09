use std::f32::consts::PI;

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
