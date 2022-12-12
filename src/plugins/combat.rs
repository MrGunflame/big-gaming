mod damage;
mod health;
mod resistances;

use bevy::prelude::{Bundle, Entity, Plugin, Query, Res, With};
use bevy_rapier3d::prelude::{RapierContext, Velocity};
pub use damage::{Damage, IncomingDamage};
pub use health::{Health, MaxHealth};
pub use resistances::{Resistance, Resistances};

use crate::components::{Actor, ActorState};

#[derive(Copy, Clone, Debug)]
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(apply_incoming_damage)
            .add_system(fall_damage);
    }
}

fn apply_incoming_damage(
    mut entities: Query<(
        &mut IncomingDamage,
        &mut Health,
        &Resistances,
        &mut ActorState,
    )>,
) {
    for (mut inc, mut health, resistances, mut state) in &mut entities {
        while let Some(damage) = inc.pop() {
            *health -= damage.amount;

            if health.is_zero() {
                *state = ActorState::Dead;
                inc.clear();
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

#[derive(Bundle)]
pub struct CombatBundle {
    pub incoming_damage: IncomingDamage,
    pub health: Health,
    pub max_health: MaxHealth,
    pub resistances: Resistances,
}

impl CombatBundle {
    pub fn new() -> Self {
        Self {
            incoming_damage: IncomingDamage::new(),
            health: Health::new(50),
            max_health: MaxHealth::new(50),
            resistances: Resistances::new(),
        }
    }
}
