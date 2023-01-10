use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use crate::id::WeakId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A unique identifer for an object.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ObjectId(pub WeakId<u32>);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Object;

#[derive(Clone, Debug, Component)]
pub struct ObjectChildren {
    pub object: Entity,
    pub children: Vec<Entity>,
}
