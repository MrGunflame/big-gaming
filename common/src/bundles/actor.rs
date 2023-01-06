use bevy_ecs::bundle::Bundle;

use super::combat::CombatBundle;
use super::transform::TransformBundle;
use super::visibility::VisibilityBundle;

#[derive(Clone, Debug, Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub transform: TransformBundle,
    #[bundle]
    pub visibility: VisibilityBundle,
    #[bundle]
    pub combat: CombatBundle,
}
