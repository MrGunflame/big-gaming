use bevy_ecs::component::Component;

/// An actor that should be AI controlled.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct AiController;

/// The threat factor that an entity poses to an AI actor.
///
/// The value is a combination of many factors including:
/// - Armor/[`Resistances`] actor
/// - Weapon of the actor (e.g. weapon type: automatic = higher threat on close range, long range
/// weapon = higher threat on long range, melee = very low threat on high range)
/// - Distance (lower range = higher threat generally)
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Component)]
#[repr(transparent)]
pub struct Threat(pub f32);

/// The perception factor of an AI actor.
///
/// The perception of an actor describes how good an actors senses are.
///
/// The percention is modified by several factors:
/// - Light/Darkness
/// - Weather effects (e.g. mist)
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Component)]
#[repr(transparent)]
pub struct Perception(pub f32);

impl Default for Perception {
    #[inline]
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Component)]
#[repr(transparent)]
pub struct Accuracy(pub f32);

impl Default for Accuracy {
    #[inline]
    fn default() -> Self {
        Self(1.0)
    }
}
