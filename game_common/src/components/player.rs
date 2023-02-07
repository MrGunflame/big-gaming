//! Player components

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

/// A marker component for a player.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Player;

/// A marker component for a [`Player`] hosting the current session.
///
/// Unlike [`Player`], which marks all players, `HostPlayer` only marks the currently controlled
/// player. There **must** only be a single entity with the `HostPlayer` component. There should
/// be no entity with the `HostPlayer` component when running the game in a server context.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct HostPlayer;

#[derive(Copy, Clone, Debug, Component)]
pub enum FocusedEntity {
    Some { entity: Entity, distance: f32 },
    Container { entity: Entity },
    None,
}
