use bytemuck::{Pod, Zeroable};

use crate::record::RecordReference;

use super::AsComponent;

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MeshInstance {
    pub id: RecordReference,
}

impl AsComponent for MeshInstance {
    const ID: RecordReference = super::MESH_INSTANCE;

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
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);

    pub const RED: Self = Self([1.0, 0.0, 0.0, 1.0]);
    pub const GREEN: Self = Self([0.0, 1.0, 0.0, 1.0]);
    pub const BLUE: Self = Self([0.0, 0.0, 1.0, 1.0]);

    pub fn as_rgb(self) -> [f32; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    #[inline]
    pub const fn from_rgb(rgb: [f32; 3]) -> Self {
        Self([rgb[0], rgb[1], rgb[2], 1.0])
    }

    #[inline]
    pub const fn from_rgba(rgba: [f32; 4]) -> Self {
        Self(rgba)
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
}

impl AsComponent for DirectionalLight {
    const ID: RecordReference = super::DIRECTIONAL_LIGHT;

    fn from_bytes(buf: &[u8]) -> Self {
        bytemuck::pod_read_unaligned(buf)
    }

    fn to_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
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
    const ID: RecordReference = super::POINT_LIGHT;

    fn from_bytes(buf: &[u8]) -> Self {
        bytemuck::pod_read_unaligned(buf)
    }

    fn to_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    /// Inner cutoff angle
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

impl AsComponent for SpotLight {
    const ID: RecordReference = super::SPOT_LIGHT;

    fn from_bytes(buf: &[u8]) -> Self {
        bytemuck::pod_read_unaligned(buf)
    }

    fn to_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}
