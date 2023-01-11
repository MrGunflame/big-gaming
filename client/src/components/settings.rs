//! Dynamic settings

use bevy::prelude::{Component, Resource, Vec3};

#[derive(Copy, Clone, Debug, Default, Resource)]
pub struct CameraSettings {
    /// Camera offset relative to the player character in third person mode.
    pub offset: Vec3,
}
