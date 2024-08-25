#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A unique entity shared between server and clients.
///
/// Note that this is distinct from [`Entity`], which is only local. `ServerEntity` is shared
/// between server and clients. It is therefore only contained on entities that need to be
/// synchronized (e.g. excluding UI components).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ServerEntity(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ServerResource(pub u64);
