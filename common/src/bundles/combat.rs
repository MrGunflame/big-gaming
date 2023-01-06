use bevy_ecs::bundle::Bundle;

use crate::components::combat::{Health, IncomingDamage, Resistances};

/// A bundle of essential components for combat.
///
/// This bundle includes:
/// - [`Health`]
/// - [`Resistances`]
/// - [`IncomingDamage`]
#[derive(Clone, Debug, Bundle, Default)]
pub struct CombatBundle {
    pub health: Health,
    pub resistances: Resistances,
    pub incoming_damage: IncomingDamage,
}
