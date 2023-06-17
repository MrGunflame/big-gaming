//! The core game systems.

use animation::AnimationPlugin;
use combat::CombatPlugin;
use game_asset::AssetPlugin;
use modules::ModulePlugin;
use world::{LevelPlugin, ObjectPlugin, SpawnPlugin, WorldTimePlugin};

use bevy_app::{App, Plugin};

pub mod animation;
pub mod combat;
pub mod modules;
pub mod movement;
pub mod time;
pub mod world;

pub mod logger;

#[derive(Copy, Clone, Debug, Default)]
pub struct CorePlugins;

impl Plugin for CorePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugin(time::TimePlugin);

        app.add_plugin(AssetPlugin::default());
        app.add_plugin(CombatPlugin);
        app.add_plugin(LevelPlugin);
        app.add_plugin(ObjectPlugin);
        app.add_plugin(WorldTimePlugin::default());
        app.add_plugin(SpawnPlugin);
        app.add_plugin(AnimationPlugin);
        app.add_plugin(ModulePlugin);
    }
}
