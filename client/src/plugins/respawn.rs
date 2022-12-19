use bevy::prelude::{Commands, Component, Entity, Plugin, Query, Transform, Vec3};
use common::components::combat::Health;

use crate::components::ActorState;

pub struct RespawnPlugin;

impl Plugin for RespawnPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(respawn);
    }
}

/// The point an entity should respawn at.
#[derive(Copy, Clone, Debug, PartialEq, Component)]
pub struct RespawnPoint(pub Vec3);

/// Actor is respawning.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Component)]
pub enum Respawn {
    /// Respawn an actor normally using their [`RespawnPoint`].
    #[default]
    Normal,
    /// Respawn the actor on place.
    OnPlace,
}

impl Respawn {
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }
}

fn respawn(
    mut commands: Commands,
    mut actors: Query<(
        Entity,
        &mut Transform,
        &mut Health,
        &mut ActorState,
        &RespawnPoint,
        &Respawn,
    )>,
) {
    for (entity, mut transform, mut health, mut state, respawn_point, respawn) in &mut actors {
        if respawn.is_normal() {
            transform.translation = respawn_point.0;
        }

        *health = Health::new(50);
        *state = ActorState::NORMAL;
        commands.entity(entity).remove::<Respawn>();
    }
}
