use crate::id::WeakId;

/// A unique identifier for a race (actor base).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RaceId(pub WeakId<u32>);
