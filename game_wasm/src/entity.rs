use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct EntityId {
    index: u64,
}

impl EntityId {
    pub const fn from_raw(index: u64) -> Self {
        Self { index }
    }

    pub const fn into_raw(self) -> u64 {
        self.index
    }
}
