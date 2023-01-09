use bevy::prelude::{Bundle, Entity, EventWriter, Plugin, Query, Res, Transform, With};
use bevy_rapier3d::prelude::{RapierContext, Velocity};
use common::components::actor::ActorState;
use common::components::animation::{AnimationId, AnimationQueue};
use common::components::combat::{Damage, Health, IncomingDamage, Resistances};

use crate::components::Actor;
use crate::ui::Focus;

#[derive(Copy, Clone, Debug)]
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(apply_incoming_damage)
            .add_system(fall_damage)
            .add_system(kill_out_of_bounds);
    }
}

fn apply_incoming_damage(
    mut entities: Query<(
        &mut IncomingDamage,
        &mut Health,
        &Resistances,
        &mut ActorState,
        Option<&mut AnimationQueue>,
    )>,
) {
    for (mut inc, mut health, resistances, mut state, mut queue) in &mut entities {
        while let Some(damage) = inc.pop() {
            *health -= damage.amount;

            if health.is_zero() {
                *state = ActorState::DEAD;
                inc.clear();

                if let Some(queue) = &mut queue {
                    queue.push(AnimationId::DEATH);
                }
            }
        }
    }
}

fn fall_damage(
    rapier: Res<RapierContext>,
    mut entities: Query<(Entity, &mut IncomingDamage, &Velocity), With<Actor>>,
) {
    for (entity, mut inc, velocity) in &mut entities {
        if velocity.linvel.y < -5.0 {
            for contact_pair in rapier.contacts_with(entity) {
                if contact_pair.has_any_active_contacts() {
                    let other_collider = if contact_pair.collider1() == entity {
                        contact_pair.collider2()
                    } else {
                        contact_pair.collider1()
                    };

                    inc.push(Damage::new(velocity.linvel.y as u32));
                }
            }
        }
    }
}

fn kill_out_of_bounds(mut entities: Query<(&Transform, &ActorState, &mut IncomingDamage)>) {
    for (transform, state, mut inc) in &mut entities {
        // IncomingDamage::push might allocate, so we only push if the actor is still alive.
        if transform.translation.y < -1000.0 && *state != ActorState::DEAD {
            inc.push(Damage::new(u32::MAX));
        }
    }
}

#[derive(Bundle)]
pub struct CombatBundle {
    pub incoming_damage: IncomingDamage,
    pub health: Health,
    pub resistances: Resistances,
}

impl CombatBundle {
    pub fn new() -> Self {
        Self {
            incoming_damage: IncomingDamage::new(),
            health: Health::new(50),
            resistances: Resistances::new(),
        }
    }
}
