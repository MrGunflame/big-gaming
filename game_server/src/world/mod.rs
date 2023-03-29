mod cell;
pub mod level;
mod terrain;

use bevy::prelude::{IntoSystemConfig, Plugin};
use game_common::world::source::StreamingSources;

use crate::plugins::tick;

use self::level::Level;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Level::new());
        app.insert_resource(StreamingSources::new());

        // app.add_system(level::update_streaming_sources.after(tick));
        // app.add_system(level::update_level.after(level::update_streaming_sources));
    }
}
