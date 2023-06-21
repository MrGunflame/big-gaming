use bevy_app::{App, Plugin};

use self::actor::load_actor;
use self::object::load_object;
use self::terrain::load_terrain;

pub mod actor;
pub mod item;
pub mod object;
pub mod terrain;

pub struct LoadEntityPlugin;

impl Plugin for LoadEntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(load_terrain);
        app.add_system(load_object);
        app.add_system(load_actor);
    }
}
