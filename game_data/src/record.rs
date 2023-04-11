use std::fmt::{self, Display, Formatter, LowerHex};

use game_common::units::Mass;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RecordId(pub u32);

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub id: RecordId,
    pub name: String,
    pub body: RecordBody,
}

#[derive(Clone, Debug)]
pub enum RecordBody {
    Item(ItemRecord),
}

#[derive(Clone, Debug)]
pub struct ItemRecord {
    pub mass: Mass,
    // TODO: Add separate Value type.
    pub value: u64,
}
