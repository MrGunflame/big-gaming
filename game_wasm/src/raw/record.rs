use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordReference {
    pub module: [u8; 16],
    pub record: u32,
}
