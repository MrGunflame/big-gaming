use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
    #[repr(transparent)]
    pub struct MaterialFlags: u32 {
        const UNLIT = 0b0000_0000_0000_0001;
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct MaterialConstants {
    pub base_color: [f32; 4],
    pub base_metallic: f32,
    pub base_roughness: f32,
    pub reflectance: f32,
    // Align to vec4<f32>.
    pub _pad: [u32; 1],
}
