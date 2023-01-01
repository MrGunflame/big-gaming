//! Loading definitions of game data
//!
//! The [`GameArchive`] serves as a central point for loading any game data.
//!
//! # Data types
//!
//! - [`Item`]: An item which may be picked up and dropped into the world by actors. Examples
//! include weapons, armor, ammo, Scrap and resources.
//!
//! - [`Object`]: Static and dynamic objects placed into the game world. Common examples for static
//! objects include walls, roads or rubble. This also includes static but interactable objects,
//! like doors and gates. Dynamic objects include objects that are affected by physics, but cannot
//! be picked up by an actor, i.e. they are not an [`Item`].
//!
//! [`Item`]: items::Item
//! [`Object`]: objects::Object
mod archive;
mod items;
mod objects;

#[cfg(feature = "json")]
mod json;

use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use bevy_ecs::system::Resource;
use parking_lot::RwLock;

use self::items::Item;
use self::json::JsonLoader;
use crate::components::items::ItemId;
use crate::id::WeakId;

/// The entrypoint of loading external data.
#[derive(Debug, Resource)]
pub struct GameArchive {
    items: RwLock<HashMap<ItemId, Arc<Item>>>,
    /// The id of the next item.
    item_id: AtomicU32,
}

impl GameArchive {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
            item_id: AtomicU32::new(0),
        }
    }

    pub fn item(&self, id: ItemId) -> Option<Ref<'_, Item>> {
        self.items().get(id)
    }

    /// Loads an archive.
    pub fn load<P>(&self, path: P)
    where
        P: AsRef<Path>,
    {
        tracing::info!("Loading {:?}", path.as_ref());

        let items = JsonLoader::new(path.as_ref());

        for item in items {
            self.items().insert(item);
        }

        tracing::info!("Loaded {:?}", path.as_ref());
    }

    #[inline]
    pub fn items(&self) -> Items<'_> {
        Items { archive: self }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Items<'a> {
    archive: &'a GameArchive,
}

impl<'a> Items<'a> {
    pub fn get(&self, id: ItemId) -> Option<Ref<'a, Item>> {
        let items = self.archive.items.read();
        items.get(&id).map(|item| Ref {
            archive: self.archive,
            item: item.clone(),
        })
    }

    pub fn insert(&self, item: Item) -> ItemId {
        let mut items = self.archive.items.write();

        let id = self.id();
        items.insert(id, Arc::new(item));
        id
    }

    pub fn remove(&self, id: ItemId) {
        let mut items = self.archive.items.write();
        items.remove(&id);
    }

    /// Generates and returns a new weak [`ItemId`].
    #[inline]
    fn id(&self) -> ItemId {
        let id = self.archive.item_id.fetch_add(1, Ordering::Relaxed);
        assert!(id != u32::MAX);
        ItemId(WeakId(id))
    }
}

#[derive(Debug)]
pub struct Ref<'a, T> {
    archive: &'a GameArchive,
    item: Arc<T>,
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}
