use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, Entity, Plugin, Query, Res, Transform, With,
};
use bevy::scene::SceneBundle;
use bevy_rapier3d::prelude::RapierContext;
use common::components::actor::{Actor, ActorLimb};
use common::components::combat::{Damage, IncomingDamage};
use common::components::object::ObjectChildren;
use common::components::projectile::Projectile;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(projectile_collision);
    }
}

fn projectile_collision(
    mut commands: Commands,
    rapier: Res<RapierContext>,
    mut projectiles: Query<(Entity, &Transform, Option<&Damage>), With<Projectile>>,
    limbs: Query<&ActorLimb>,
    mut actors: Query<&mut IncomingDamage, With<Actor>>,
    mut objects: Query<&mut ObjectChildren>,
    assets: Res<AssetServer>,
) {
    for (entity, transform, damage) in &mut projectiles {
        // If a projectile collides with multiple entities we apply the
        // damage to all entities.
        for contact_pair in rapier.contacts_with(entity) {
            let target = if contact_pair.collider1() == entity {
                contact_pair.collider2()
            } else {
                contact_pair.collider1()
            };

            if let Some(damage) = damage {
                if let Ok(limb) = limbs.get(target) {
                    actors
                        .get_mut(limb.actor)
                        .expect("actor without IncomingDamage")
                        .push(*damage);
                } else if let Ok(mut children) = objects.get_mut(entity) {
                    let id = commands
                        .spawn(SceneBundle {
                            scene: assets.load("impact.glb#Scene0"),
                            transform: *transform,
                            ..Default::default()
                        })
                        .id();

                    children.children.push(id);
                }
            }
        }

        // Despawn the projectile.
        commands.entity(entity).despawn_recursive();
    }
}
