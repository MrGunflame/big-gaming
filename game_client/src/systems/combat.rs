use bevy::prelude::{Commands, Component, Entity, EventReader, Query};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct Health(u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Limb {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

/// The raw damage an entity does once it hits an actor.
///
/// Note that this is the raw damage, which does not include any resistances.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct Damage(u32);

/// Despawn killed entities.
pub fn despawn_killed(mut commands: Commands, mut entities: Query<(Entity, &Health)>) {
    for (entity, health) in &mut entities {
        if health.0 == 0 {
            commands.entity(entity).despawn();
        }
    }
}
