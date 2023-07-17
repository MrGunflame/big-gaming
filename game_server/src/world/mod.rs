pub mod level;
mod terrain;

use bevy_app::{App, Plugin};
use game_common::world::gen::Generator;
use game_common::world::source::StreamingSources;

use crate::ServerState;

use self::level::Level;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let state: ServerState = app.world.remove_resource().unwrap();
        let gen = Generator::from(state.generator);

        app.insert_resource(Level::new(gen));
        app.insert_resource(StreamingSources::new());
    }
}
