use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct DirectionalLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct PointLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}
