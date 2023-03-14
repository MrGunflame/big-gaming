use bevy::prelude::{Commands, Entity, Query, Without};
use bevy_rapier3d::prelude::{Collider, RigidBody};
use game_common::components::items::LoadItem;
use game_common::components::terrain::LoadTerrain;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TerrainPlugin;

impl bevy::prelude::Plugin for TerrainPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(load_terrain_collider);
        app.add_system(load_item_collider);
    }
}

fn load_terrain_collider(
    mut commands: Commands,
    meshes: Query<(Entity, &LoadTerrain), Without<Collider>>,
) {
    for (entity, mesh) in &meshes {
        commands
            .entity(entity)
            .insert(RigidBody::Fixed)
            .insert(mesh.mesh.collider());

        commands.entity(entity).remove::<LoadTerrain>();
    }
}

fn load_item_collider(
    mut commands: Commands,
    items: Query<(Entity, &LoadItem), Without<Collider>>,
) {
    for (entity, item) in &items {
        commands
            .entity(entity)
            .insert(RigidBody::Dynamic)
            .insert(Collider::cuboid(1.0, 1.0, 1.0));

        commands.entity(entity).remove::<LoadItem>();
    }
}
