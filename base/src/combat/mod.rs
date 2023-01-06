use bevy::prelude::{Plugin, Query};
use common::components::actor::ActorState;
use common::components::combat::{Health, IncomingDamage, Resistances};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(apply_incoming_damage);
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
