pub mod settings;

use std::f32::consts::PI;

use bevy::prelude::{Component, EulerRot, Mat3, Quat, Vec3};

use crate::utils::Radians;

/// A entity that exists within the game world.
///
/// This only includes that exist within the world, i.e. excludes components like cameras, markers,
/// etc...
#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct WorldObject;

/// A temporary [`WorldObject`] that does not consist between runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct TemporaryObject;

/// A permanent [`WorldObject`] that should be resumed between runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct PersistentObject;

/// An object that can be interacted with.
#[derive(Clone, Debug)]
pub struct Interactable {
    pub name: Option<String>,
}

/// A entitiy that can act within a world.
///
/// Unlike static props, `Actor` is a marker component for all entities that may act on its own,
/// or be acted upon. This most notably includes player characters and NPCs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct Actor;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct Player;
