use bevy::prelude::{
    Assets, Color, Commands, Entity, Handle, Mesh, PbrBundle, Quat, Query, ResMut,
    StandardMaterial, Transform, Vec3, Without,
};
use game_common::components::terrain::LoadTerrain;

pub fn load_terrain_mesh(
    mut commands: Commands,
    meshes: Query<(Entity, &LoadTerrain), Without<Handle<Mesh>>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mesh) in &meshes {
        commands.entity(entity).insert(PbrBundle {
            mesh: mesh_assets.add(mesh.mesh.mesh()),
            material: materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            }),
            transform: Transform {
                translation: mesh.mesh.cell.min(),
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(1.0),
            },
            ..Default::default()
        });
    }
}
