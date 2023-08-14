//! Player components

/// A marker component for a player.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Player;

/// A marker component for a [`Player`] hosting the current session.
///
/// Unlike [`Player`], which marks all players, `HostPlayer` only marks the currently controlled
/// player. There **must** only be a single entity with the `HostPlayer` component. There should
/// be no entity with the `HostPlayer` component when running the game in a server context.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostPlayer;
