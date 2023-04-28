use std::fmt::{self, Display, Formatter, LowerHex};

use bytemuck::{Pod, Zeroable};

use crate::module::ModuleId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct RecordId(pub u32);

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct RecordReference {
    pub module: ModuleId,
    pub record: RecordId,
}

impl RecordReference {
    pub const STUB: Self = Self {
        module: ModuleId::CORE,
        record: RecordId(0),
    };
}

impl Display for RecordReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.module, self.record)
    }
}
