use glam::{EulerRot, Quat};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// An extension trait for types that can be interpreted as a rotation.
///
/// `RotationExt` exposes functions to operate on simpler yaw-pitch-roll euler angles.
pub trait RotationExt {
    /// Returns the yaw value.
    fn yaw(&self) -> f32;

    /// Returns the pitch value.
    fn pitch(&self) -> f32;

    /// Sets the yaw value to `yaw`.
    fn set_yaw(&mut self, yaw: f32);

    /// Sets the pitch value to `pitch`.
    fn set_pitch(&mut self, pitch: f32);

    /// Creates a new copy of the type with the given `yaw` value.
    #[inline]
    fn with_yaw(mut self, yaw: f32) -> Self
    where
        Self: Copy,
    {
        self.set_yaw(yaw);
        self
    }

    /// Creates a new copy of the type with the given `pitch` value.
    #[inline]
    fn with_pitch(mut self, pitch: f32) -> Self
    where
        Self: Copy,
    {
        self.set_pitch(pitch);
        self
    }
}

impl RotationExt for Quat {
    #[inline]
    fn yaw(&self) -> f32 {
        let (y, _, _) = self.to_euler(EulerRot::YXZ);
        y
    }

    #[inline]
    fn pitch(&self) -> f32 {
        let (_, x, _) = self.to_euler(EulerRot::YXZ);
        x
    }

    #[inline]
    fn set_yaw(&mut self, yaw: f32) {
        let (mut y, x, z) = self.to_euler(EulerRot::YXZ);
        y += yaw;
        *self = Self::from_euler(EulerRot::YXZ, y, x, z);
    }

    #[inline]
    fn set_pitch(&mut self, pitch: f32) {
        let (y, mut x, z) = self.to_euler(EulerRot::YXZ);
        x += pitch;
        *self = Self::from_euler(EulerRot::YXZ, y, x, z);
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::{EulerRot, Quat, RotationExt};

    /// Asserts approximated equals on two f32s.
    macro_rules! assert_f32 {
        ($a:expr, $b:expr) => {{
            if !($a >= $b - 0.0001 && $a <= $b + 0.0001) {
                // Fallback to the assert_eq macro for better output.
                assert_eq!($a, $b);
            }
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
    fn test_quat_get() {
        let quat = Quat::IDENTITY;
        assert_f32!(quat.yaw(), 0.0);
        assert_f32!(quat.pitch(), 0.0);

        let quat = Quat::from_euler(EulerRot::YXZ, PI / 2.0, 0.0, 0.0);
        assert_f32!(quat.yaw(), PI / 2.0);
        assert_f32!(quat.pitch(), 0.0);

        let quat = Quat::from_euler(EulerRot::YXZ, PI / 2.0, PI / 4.0, 0.0);
        assert_f32!(quat.yaw(), PI / 2.0);
        assert_f32!(quat.pitch(), PI / 4.0);

        let quat = Quat::from_euler(EulerRot::YXZ, PI / 2.0, PI / 4.0, PI / 2.0);
        assert_f32!(quat.yaw(), PI / 2.0);
        assert_f32!(quat.pitch(), PI / 4.0);
    }

    #[test]
    fn test_quat_set() {
        let mut quat = Quat::IDENTITY;
        assert_f32!(quat.yaw(), 0.0);
        assert_f32!(quat.pitch(), 0.0);

        quat.set_yaw(PI / 2.0);
        assert_f32!(quat.yaw(), PI / 2.0);
        assert_f32!(quat.pitch(), 0.0);

        quat.set_pitch(PI / 4.0);
        assert_f32!(quat.yaw(), PI / 2.0);
        assert_f32!(quat.pitch(), PI / 4.0);

        quat.set_pitch(PI / 2.0);
        assert_f32!(quat.yaw(), PI / 2.0);
        assert_f32!(quat.pitch(), PI / 2.0);
    }
}
