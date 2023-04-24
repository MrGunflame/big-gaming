use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct InventoryId(u64);
