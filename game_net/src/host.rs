//! Hosts
//!
//! A host is an entity that captures the world space around itself. Most hosts are players.

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HostId(pub u32);
