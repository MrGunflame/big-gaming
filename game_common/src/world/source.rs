//! Streaming source
//!
use bevy_ecs::component::Component;

/// An entity that (un)loads cells as it moves.
#[derive(Clone, Debug, Component)]
pub struct StreamingSource {}

impl StreamingSource {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for StreamingSource {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
