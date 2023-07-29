use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Vec3};
use wgpu::{Buffer, IndexFormat};

pub struct BufferVec<T: Pod> {
    vec: Vec<T>,
    buffer: Option<Buffer>,
}

#[derive(Debug)]

pub struct IndexBuffer {
    pub buffer: Buffer,
    pub format: IndexFormat,
    /// Length of the buffer, in elements.
    pub len: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Vec3F32 {
    x: f32,
    y: f32,
    z: f32,
    _pad: u32,
}

impl From<Vec3> for Vec3F32 {
    fn from(vec: Vec3) -> Self {
        Self {
            x: vec.x,
            y: vec.y,
            z: vec.z,
            _pad: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Mat3F32 {
    x_axis: [f32; 3],
    _pad0: u32,
    y_axis: [f32; 3],
    _pad1: u32,
    z_axis: [f32; 3],
    _pad2: u32,
}

impl From<Mat3> for Mat3F32 {
    fn from(mat: Mat3) -> Self {
        Self {
            x_axis: mat.x_axis.to_array(),
            y_axis: mat.y_axis.to_array(),
            z_axis: mat.z_axis.to_array(),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}
