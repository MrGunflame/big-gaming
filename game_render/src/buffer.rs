use bytemuck::Pod;
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
