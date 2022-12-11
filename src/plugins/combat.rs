mod damage;
mod health;
mod resistances;

use bevy::prelude::{Bundle, Plugin, Query};
pub use damage::{Damage, IncomingDamage};
pub use health::{Health, MaxHealth};
pub use resistances::{Resistance, Resistances};

use crate::components::ActorState;

#[derive(Copy, Clone, Debug)]
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(apply_incoming_damage);
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
