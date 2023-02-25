//! The core game systems.

use animation::AnimationPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::gltf::GltfPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::{AddAsset, AssetPlugin, CorePlugin, HierarchyPlugin, Mesh, Plugin};
use bevy::scene::ScenePlugin;
use bevy::time::TimePlugin;
use bevy::transform::TransformPlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};
use combat::CombatPlugin;
use game_audio::AudioPlugin;
use movement::MovementPlugin;
use projectile::ProjectilePlugin;
use world::{LevelPlugin, ObjectPlugin, SpawnPlugin};

pub mod ai;
pub mod animation;
pub mod combat;
pub mod debug;
pub mod movement;
pub mod projectile;
pub mod world;

#[derive(Copy, Clone, Debug, Default)]
pub struct CorePlugins;

impl Plugin for CorePlugins {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(CorePlugin::default());
        app.add_plugin(LogPlugin::default());
        app.add_plugin(TimePlugin);
        app.add_plugin(TransformPlugin);
        app.add_plugin(HierarchyPlugin);
        app.add_plugin(DiagnosticsPlugin);
        app.add_plugin(AssetPlugin::default());
        app.add_plugin(ScenePlugin);
        app.add_plugin(GltfPlugin);
        app.add_plugin(CombatPlugin);
        app.add_plugin(LevelPlugin);
        app.add_plugin(ObjectPlugin);
        app.add_plugin(world::TimePlugin::default());
        app.add_plugin(AudioPlugin::new());
        app.add_plugin(SpawnPlugin);
        // app.add_plugin(MovementPlugin);
        app.add_plugin(ProjectilePlugin);
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugin(AnimationPlugin);
        app.add_asset::<Mesh>();
    }
}
