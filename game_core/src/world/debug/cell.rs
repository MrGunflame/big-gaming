//! Render cell borders

use bevy::prelude::{
    shape, Assets, Color, Commands, Component, DespawnRecursiveExt, Entity, Mesh, PbrBundle, Query,
    ResMut, StandardMaterial, Transform, Vec3, With,
};
use game_common::components::player::HostPlayer;
use game_common::world::{CellId, CELL_SIZE};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct CellFrame;

pub fn render_cell_borders(
    mut commands: Commands,
    mut players: Query<&Transform, With<HostPlayer>>,
    mut frames: Query<Entity, With<CellFrame>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(transform) = players.get_single() else {
        return;
    };

    for frame in &frames {
        commands.entity(frame).despawn_recursive();
    }

    let cell = CellId::from(transform.translation);

    let xy = shape::Box {
        min_x: 0.0,
        max_x: CELL_SIZE.x,
        min_y: 0.0,
        max_y: CELL_SIZE.y,
        min_z: 0.0,
        max_z: 0.0,
    };

    let zy = shape::Box {
        min_x: 0.0,
        max_x: 0.0,
        min_y: 0.0,
        max_y: CELL_SIZE.y,
        min_z: 0.0,
        max_z: CELL_SIZE.z,
    };

    let xz = shape::Box {
        min_x: 0.0,
        max_x: CELL_SIZE.x,
        min_y: 0.0,
        max_y: 0.0,
        min_z: 0.0,
        max_z: CELL_SIZE.z,
    };

    // PT (0|0)
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(xy.into()),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(Vec3::new(
                cell.min_x(),
                cell.min_y(),
                cell.min_z(),
            )),
            ..Default::default()
        })
        .insert(CellFrame);

    // PT (1|0)
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(zy.into()),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(Vec3::new(
                cell.min_x(),
                cell.min_y(),
                cell.min_z(),
            )),
            ..Default::default()
        })
        .insert(CellFrame);

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(xy.into()),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(Vec3::new(
                cell.min_x(),
                cell.min_y(),
                cell.max_z(),
            )),
            ..Default::default()
        })
        .insert(CellFrame);

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(zy.into()),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(Vec3::new(
                cell.max_x(),
                cell.min_y(),
                cell.min_z(),
            )),
            ..Default::default()
        })
        .insert(CellFrame);

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(xz.into()),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(Vec3::new(
                cell.min_x(),
                cell.min_y(),
                cell.min_z(),
            )),
            ..Default::default()
        })
        .insert(CellFrame);

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(xz.into()),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(Vec3::new(
                cell.min_x(),
                cell.max_y(),
                cell.min_z(),
            )),
            ..Default::default()
        })
        .insert(CellFrame);
}
