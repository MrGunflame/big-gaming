//! Dynamic settings

use bevy::prelude::{Resource, Vec3};

#[derive(Copy, Clone, Debug, Resource)]
pub struct CameraSettings {
    /// Camera offset relative to the player character in third person mode.
    pub offset: Vec3,
    /// Head movement (bumping) factor in first person. (0 to disable entirely)
    pub head_bumping: f32,
}

impl CameraSettings {
    pub const fn new() -> Self {
        Self {
            offset: Vec3::new(0.0, 1.6, 0.0),
            head_bumping: 1.0,
        }
    }
}

impl Default for CameraSettings {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
