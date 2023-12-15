use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

use crate::record::{ModuleId, RecordId, RecordReference};

use super::AsComponent;

macro_rules! define_id {
    ($($id:ident => $val:expr),*,) => {
        $(
            const $id: RecordReference = RecordReference {
                module: ModuleId::CORE,
                record: RecordId($val),
            };
        )*
    };
}

define_id! {
    TRANSFORM => 1,

    // Rendering
    MESH_INSTANCE => 2,
    DIRECTIONAL_LIGHT => 3,
    POINT_LIGHT => 4,
    SPOT_LIGHT => 5,

    // Physics
    RIGID_BODY => 6,
    COLLIDER => 7,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl AsComponent for Transform {
    const ID: RecordReference = TRANSFORM;

    fn from_bytes(buf: &[u8]) -> Self {
        let translation: [f32; 3] = bytemuck::pod_read_unaligned(&buf[0..4 * 3]);
        let rotation: [f32; 4] = bytemuck::pod_read_unaligned(&buf[4 * 3..4 * 3 + 4 * 4]);
        let scale: [f32; 3] = bytemuck::pod_read_unaligned(&buf[4 * 7..4 * 7 + 3 * 4]);

        Self {
            translation: Vec3::from_array(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from_array(scale),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let translation = self.translation.to_array();
        let rotation = self.rotation.to_array();
        let scale = self.scale.to_array();

        let mut bytes = Vec::new();
        bytes.extend(bytemuck::bytes_of(&translation));
        bytes.extend(bytemuck::bytes_of(&rotation));
        bytes.extend(bytemuck::bytes_of(&scale));
        bytes
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

impl AsComponent for PointLight {
    const ID: RecordReference = POINT_LIGHT;

    fn from_bytes(buf: &[u8]) -> Self {
        bytemuck::pod_read_unaligned(buf)
    }

    fn to_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);
}
