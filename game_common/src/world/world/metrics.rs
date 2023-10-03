use game_common::metrics::Gauge;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WorldMetrics {
    /// The total number of snapshots in the world.
    pub snapshots: Gauge,
    /// The total number of entities in the world.
    pub entities: Gauge,
    /// The number of current buffered deltas across all snapshots across all cells.
    pub deltas: Gauge,
}

impl WorldMetrics {
    pub(crate) const fn new() -> Self {
        Self {
            snapshots: Gauge::new(),
            entities: Gauge::new(),
            deltas: Gauge::new(),
        }
    }
}
