pub mod level;
mod terrain;

use bevy_app::{App, Plugin};
use game_common::world::source::StreamingSources;

use self::level::Level;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Level::default());
        app.insert_resource(StreamingSources::new());
    }
}
