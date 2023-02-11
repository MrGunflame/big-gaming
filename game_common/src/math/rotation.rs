use glam::{Quat, Vec3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// An extension trait for types that can be interpreted as a rotation.
pub trait RotationExt {
    /// Returns a direction unit vector represented by this rotation.
    fn dir_vec(&self) -> Vec3;
}

impl RotationExt for Quat {
    #[inline]
    fn dir_vec(&self) -> Vec3 {
        *self * -Vec3::Z
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use glam::Vec3;

    use super::{Quat, RotationExt};

    /// Asserts approximated equals on two f32s.
    macro_rules! assert_f32 {
        ($a:expr, $b:expr) => {{
            if !($a >= $b - 0.0001 && $a <= $b + 0.0001) {
                // Fallback to the assert_eq macro for better output.
                assert_eq!($a, $b);
            }
        }};
    }

    macro_rules! assert_vec {
        ($a:expr, $b:expr) => {{
            assert_f32!($a.x, $b.x);
            assert_f32!($a.y, $b.y);
            assert_f32!($a.z, $b.z);
        }};
    }

    #[test]
    fn test_assert_f32() {
        assert_f32!(0.0001, 0.0002);
    }

    #[test]
    #[should_panic]
    fn test_assert_f32_failure() {
        assert_f32!(0.001, 0.002);
    }

    #[test]
    fn test_quat_dir_vec() {
        let quat = Quat::IDENTITY;
        assert_eq!(quat.dir_vec(), Vec3::new(0.0, 0.0, -1.0));

        let quat = Quat::from_axis_angle(Vec3::Y, PI / 2.0);
        assert_vec!(quat.dir_vec(), Vec3::new(-1.0, 0.0, 0.0));

        let quat = Quat::from_axis_angle(Vec3::X, PI / 2.0);
        assert_vec!(quat.dir_vec(), Vec3::new(0.0, 1.0, 0.0));
    }
}
