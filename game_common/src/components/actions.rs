use std::fmt::Debug;

use bytemuck::{Pod, Zeroable};

use crate::record::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct ActionId(pub RecordReference);
