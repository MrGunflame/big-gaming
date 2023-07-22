//! The core game systems.

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

use combat::CombatPlugin;
use game_asset::AssetPlugin;
use hierarchy::HierarchyPlugin;
use modules::ModulePlugin;
use transform::TransformPlugin;
use world::{LevelPlugin, ObjectPlugin, SpawnPlugin, WorldTimePlugin};

use bevy_app::{App, Plugin};

pub mod combat;
pub mod counter;
pub mod hierarchy;
pub mod modules;
pub mod time;
pub mod transform;
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
        app.add_plugin(ModulePlugin);
    }
}
