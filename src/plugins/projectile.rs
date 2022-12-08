use bevy::prelude::{
    App, Commands, CoreStage, DespawnRecursiveExt, Entity, Plugin, Query, Res, ResMut, With, World,
};
use bevy_rapier3d::prelude::RapierContext;

use crate::entities::projectile::Projectile;

use super::combat::{Damage, IncomingDamage};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(despawn_collided_projectiles);
    }
}

/// Despawn any [`Projectile`]s colliding with another entity.
fn despawn_collided_projectiles(
    mut commands: Commands,
    rapier: Res<RapierContext>,
    mut entities: Query<(Entity, Option<&Damage>), With<Projectile>>,
) {
    for (entity, damage) in &mut entities {
        for contact_pair in rapier.contacts_with(entity) {
            if contact_pair.has_any_active_contacts() {
                let other_collider = if contact_pair.collider1() == entity {
                    contact_pair.collider2()
                } else {
                    contact_pair.collider1()
                };

                if let Some(damage) = damage {
                    todo!()
                }

                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
