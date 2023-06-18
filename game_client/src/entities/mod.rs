use bevy_app::{App, Plugin};

use self::terrain::load_terrain;

pub mod actor;
pub mod item;
pub mod object;
pub mod terrain;

pub struct LoadEntityPlugin;

impl Plugin for LoadEntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(load_terrain);
    }
}
