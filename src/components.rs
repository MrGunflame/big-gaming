use bevy::prelude::{EulerRot, Mat3, Vec3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rotation {
    mat: Mat3,
}

impl Rotation {
    // pub fn new() -> Self {
    //     Self {}
    // }

    // pub fn with_yaw(mut self, yaw: f32) -> Self {
    //     // let mat = Mat3::from_euler(EulerRot::XYZ, yaw, 0.0, c)
    // }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rotation_yaw() {}
}
