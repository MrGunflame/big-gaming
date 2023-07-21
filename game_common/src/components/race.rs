use crate::record::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RaceId(pub RecordReference);
