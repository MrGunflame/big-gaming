use bytemuck::Pod;
use wgpu::Buffer;

pub struct BufferVec<T: Pod> {
    vec: Vec<T>,
    buffer: Option<Buffer>,
}
