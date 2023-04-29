use bytemuck::{Pod, Zeroable};

use crate::record::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct ComponentId(pub RecordReference);
