use bevy_ecs::component::Component;

/// An [`Entity`] that exists within the game world.
///
/// This only includes entities that exist within the world, i.e. excludes components like cameras,
/// markers, UI, etc..
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct WorldObject;

/// A [`WorldObject`] of low importance that should not be saved between runs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct TemporaryObject;

/// A [`WorldObject`] of high importance that should be saved between runs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct PersistentObject;
