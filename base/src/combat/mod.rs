use bevy::prelude::{
    AssetServer, Commands, Entity, EulerRot, Plugin, Query, Res, Transform, Vec3, With,
};
use bevy_rapier3d::prelude::{QueryFilter, RapierContext};
use common::bundles::ProjectileBundle;
use common::components::actor::{ActorFigure, ActorState};
use common::components::animation::{Bone, Skeleton};
use common::components::combat::{Attack, Damage, Health, IncomingDamage, Reload, Resistances};
use common::components::faction::ActorFactions;
use common::components::inventory::{Equipment, EquipmentSlot, Inventory};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(apply_incoming_damage)
            .add_system(handle_attack_events)
            .add_system(handle_reload_events);
    }
}

fn apply_incoming_damage(
    mut actors: Query<(
        &mut IncomingDamage,
        &mut Health,
        &Resistances,
        &mut ActorState,
    )>,
) {
    for (mut incoming_damage, mut health, resistances, mut state) in actors.iter_mut() {
        while let Some(damage) = incoming_damage.pop() {
            *health -= damage.amount;

            if health.health == 0 {
                *state = ActorState::DEAD;
                incoming_damage.clear();

                // The actor is already dead, no need to process any more damage events.
                break;
            }
        }
    }
}

fn handle_attack_events(
    mut commands: Commands,
    rapier: Res<RapierContext>,
    assets: Res<AssetServer>,
    mut actors: Query<(Entity, &Transform, &ActorFigure, &mut Equipment), With<Attack>>,
) {
    for (entity, transform, figure, mut equipment) in &mut actors {
        let item = match equipment.get_mut(EquipmentSlot::MAIN_HAND) {
            Some(item) => item,
            None => continue,
        };

        // Out of ammo, cannot attack.
        if !item.magazine.decrement() {
            continue;
        }

        let ray_origin = transform.translation + figure.eyes;
        let (y, x, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let ray_dir = Vec3::new(-y.sin() * x.cos(), x.sin(), -y.cos() * x.cos());
        let max_toi = 1000.0;

        let toi = match rapier.cast_ray(ray_origin, ray_dir, max_toi, true, QueryFilter::new()) {
            Some((_, toi)) => toi,
            None => max_toi,
        };

        let target = ray_origin + toi * ray_dir;

        // Create a new entity at the same position as the player,
        // pointing at the same direction as the player and a positive velocity
        // into the direction of the player.
        let mut origin = transform.translation;
        origin.y += 3.0;

        // Normalized velocity from origin towards target.
        let linvel = Vec3::normalize(target - origin) * Vec3::splat(1000.0);

        let mut projectile = ProjectileBundle::new(&assets).at(origin).looking_at(target);
        projectile.physics.velocity.linvel = linvel;

        commands.spawn(projectile).insert(Damage::new(1));
        commands.entity(entity).remove::<Attack>();
    }
}

fn handle_reload_events(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Equipment), With<Reload>>,
) {
    for (entity, mut equipment) in &mut actors {
        if let Some(item) = equipment.get_mut(EquipmentSlot::MAIN_HAND) {
            item.magazine.set(30);
        }

        commands.entity(entity).remove::<Reload>();
    }
}
