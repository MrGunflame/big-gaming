/// An [`Entity`] that exists within the game world.
///
/// This only includes entities that exist within the world, i.e. excludes components like cameras,
/// markers, UI, etc..
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldObject;

/// A [`WorldObject`] of low importance that should not be saved between runs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemporaryObject;

/// A [`WorldObject`] of high importance that should be saved between runs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersistentObject;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityName(String);

impl EntityName {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EntityName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
