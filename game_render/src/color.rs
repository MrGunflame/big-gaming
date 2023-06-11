use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct Color(pub [f32; 4]);
