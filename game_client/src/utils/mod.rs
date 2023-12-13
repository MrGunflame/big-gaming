use std::f32::consts::PI;

use glam::{Quat, Vec3};

pub fn extract_actor_rotation(rotation: Quat) -> Quat {
    let mut pt = rotation * -Vec3::Z;

    if pt.y == 1.0 {
        return rotation;
    }

    pt.y = 0.0;
    if !pt.is_normalized() {
        pt = pt.normalize();
    }

    let b = Vec3::Z;

    let mut angle = f32::clamp(pt.dot(b), -1.0, 1.0).acos();
    if pt.x.is_sign_negative() {
        angle = -angle;
    }

    let res = Quat::from_axis_angle(Vec3::Y, angle + PI);
    debug_assert!(!res.is_nan());
    res
}

#[cfg(test)]
mod tests {
    use glam::Quat;

    use crate::utils::extract_actor_rotation;

    #[test]
    fn extract_actor_rotation_nan() {
        let input = Quat::from_xyzw(3.53899e-6, -1.0000173, -0.0050006974, -0.0007022657);
        assert!(!extract_actor_rotation(input).is_nan());
    }
}
