use bevy::prelude::{AssetServer, Commands, Entity, Plugin, Query, Res, Transform, Vec3, With};
use game_common::bundles::ProjectileBundle;
use game_common::components::actor::{ActorFigure, ActorFlag, ActorFlags, Death};
use game_common::components::combat::{
    Attack, Damage, Health, IncomingDamage, Reload, Resistances,
};
use game_common::components::inventory::{Equipment, EquipmentSlot};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(apply_incoming_damage)
            // .add_system(handle_attack_events)
            .add_system(handle_reload_events)
            .add_system(remove_death);
    }
}

fn apply_incoming_damage(
    mut commands: Commands,
    mut actors: Query<(
        Entity,
        &mut IncomingDamage,
        &mut Health,
        &Resistances,
        &mut ActorFlags,
    )>,
) {
    for (entity, mut incoming_damage, mut health, resistances, mut flags) in actors.iter_mut() {
        while let Some(damage) = incoming_damage.pop() {
            *health -= damage.amount;

            if health.health == 0 {
                flags.insert(ActorFlag::DEAD);

                for flag in [
                    ActorFlag::CAN_MOVE,
                    ActorFlag::CAN_ROTATE,
                    ActorFlag::CAN_ATTACK,
                ] {
                    flags.remove(flag);
                }

                incoming_damage.clear();

                commands.entity(entity).insert(Death);

                // The actor is already dead, no need to process any more damage events.
                break;
            }
        }
    }
}

// fn handle_attack_events(
//     mut commands: Commands,
//     assets: Res<AssetServer>,
//     mut actors: Query<(Entity, &Transform, &ActorFigure, &mut Equipment, &Attack)>,
// ) {
//     for (entity, transform, figure, mut equipment, attack) in &mut actors {
//         let item = match equipment.get_mut(EquipmentSlot::MAIN_HAND) {
//             Some(item) => item,
//             None => continue,
//         };

//         if !item.cooldown.tick() {
//             continue;
//         }

//         if let Some(magazine) = &mut item.magazine {
//             // Out of ammo, cannot attack.
//             if magazine.pop().is_none() {
//                 continue;
//             }
//         }

//         // let ray_origin = transform.translation + figure.eyes;
//         // let (y, x, _) = transform.rotation.to_euler(EulerRot::YXZ);
//         // let ray_dir = Vec3::new(-y.sin() * x.cos(), x.sin(), -y.cos() * x.cos());
//         // let max_toi = 1000.0;

//         // let toi = match rapier.cast_ray(ray_origin, ray_dir, max_toi, true, QueryFilter::new()) {
//         //     Some((_, toi)) => toi,
//         //     None => max_toi,
//         // };

//         let target = attack.target;

//         // Create a new entity at the same position as the player,
//         // pointing at the same direction as the player and a positive velocity
//         // into the direction of the player.
//         let mut origin = transform.translation;
//         origin.y += 3.0;

//         // Normalized velocity from origin towards target.
//         let linvel = Vec3::normalize(target - origin) * Vec3::splat(1000.0);

//         let mut projectile = ProjectileBundle::new(&assets).at(origin).looking_at(target);
//         projectile.physics.velocity.linvel = linvel;

//         commands.spawn(projectile).insert(Damage::new(1));
//         commands.entity(entity).remove::<Attack>();
//     }
// }

fn handle_reload_events(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Equipment), With<Reload>>,
) {
    for (entity, mut equipment) in &mut actors {
        if let Some(item) = equipment.get_mut(EquipmentSlot::MAIN_HAND) {
            // if let Some(id) = item.ammo {
            //     item.magazine.as_mut().unwrap().push(id, 30);
            // }
        }

        commands.entity(entity).remove::<Reload>();
    }
}

// Remove the death event from all actors.
fn remove_death(mut commands: Commands, mut actors: Query<Entity, With<Death>>) {
    for entity in &mut actors {
        commands.entity(entity).remove::<Death>();
    }
}
