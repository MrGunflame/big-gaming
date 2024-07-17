use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use game_tracing::trace_span;

#[derive(Clone, Debug)]
pub(crate) struct FpsLimiter {
    timestep: Option<Duration>,
    last_update: Instant,
}

impl FpsLimiter {
    pub fn new(limit: FpsLimit) -> Self {
        let timestep = limit.0.map(|v| Duration::from_secs(1) / v.get());

        Self {
            timestep,
            last_update: Instant::now(),
        }
    }

    /// Blocks the calling thread until a new frame should be presented.
    pub fn block_until_ready(&mut self) {
        let _span = trace_span!("FpsLimiter::block_until_ready").entered();

        if let Some(timestep) = self.timestep {
            let now = Instant::now();
            let elapsed = now - self.last_update;

            if elapsed < timestep {
                std::thread::sleep(timestep - elapsed);
            }

            self.last_update += timestep;
        }
    }
}

/// An artificial FPS limit for the [`Renderer`].
///
/// [`Renderer`]: super::Renderer
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FpsLimit(Option<NonZeroU32>);

impl FpsLimit {
    /// The `FpsLimit` value representing no FPS limit.
    ///
    /// Frames will always be presented immediately when they are ready.
    ///
    /// This is the default for `FpsLimit`.
    pub const UNLIMITED: Self = Self(None);

    /// Creates a new `FpsLimit` with the given `limit` of frames per second.
    ///
    /// Frames will be timed to reach the value given by `limit`. If rendering is faster than
    /// `limit` rendered frames will be delayed before presenting.
    #[inline]
    #[must_use]
    pub fn limited(limit: NonZeroU32) -> Self {
        Self(Some(limit))
    }
}
