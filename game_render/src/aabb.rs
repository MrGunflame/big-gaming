use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::system::{Commands, Query, Res};
use game_asset::{Assets, Handle};
use glam::Vec3;

use crate::mesh::Mesh;

#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct Aabb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl Aabb {
    pub fn min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.half_extents
    }

    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        let center = (min + max) * 0.5;
        let half_extents = (max - min) * 0.5;

        Self {
            center,
            half_extents,
        }
    }
}

pub fn update_aabb(
    mut commands: Commands,
    entities: Query<(Entity, &Handle<Mesh>), Or<(Added<Handle<Mesh>>, Changed<Handle<Mesh>>)>>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, mesh) in &entities {
        let mesh = meshes.get(mesh.id()).unwrap();

        if let Some(aabb) = mesh.compute_aabb() {
            commands.entity(entity).insert(aabb);
        }
    }
}
