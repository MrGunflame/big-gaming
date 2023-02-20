use bevy_ecs::bundle::Bundle;

use crate::components::actor::Actor;
use crate::components::inventory::{Equipment, Inventory};

use super::combat::CombatBundle;
use super::transform::TransformBundle;
use super::visibility::VisibilityBundle;

#[derive(Clone, Debug, Default, Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub transform: TransformBundle,
    #[bundle]
    pub visibility: VisibilityBundle,
    #[bundle]
    pub combat: CombatBundle,

    pub actor: Actor,

    pub inventory: Inventory,
    pub equipment: Equipment,
}
