//! Ingame time systems
use std::time::Duration;

use bevy::prelude::{Plugin, Res, ResMut};
use bevy::time::Time;
use game_common::world::time::{DateTime, TimeScale};

/// Driver for the ingame clock.
#[derive(Clone, Debug, Default)]
pub struct TimePlugin {
    /// The starting [`DateTime`] when the time system is first initialized.
    pub start: DateTime,
    /// The scale at which time passes.
    pub scale: TimeScale,
}

impl TimePlugin {
    /// Creates a new `TimePlugin` starting at the given starting [`DateTime`] and using the
    /// default [`TimeScale`].
    #[inline]
    pub fn new(start: DateTime) -> Self {
        Self {
            start,
            scale: TimeScale::default(),
        }
    }
}

impl Plugin for TimePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(self.start)
            .insert_resource(self.scale)
            .add_system(advance_time);
    }
}

fn advance_time(time: Res<Time>, mut datetime: ResMut<DateTime>, scale: Res<TimeScale>) {
    let nsecs = time.delta().as_nanos() as f32 * scale.0;
    *datetime += Duration::from_nanos(nsecs as u64);
}
