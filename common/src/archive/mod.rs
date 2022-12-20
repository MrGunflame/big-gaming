mod archive;
mod items;

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
        let items = self.items.read();
        items.get(&id).map(|item| Ref {
            archive: self,
            item: item.clone(),
        })
    }

    /// Loads an archive.
    pub fn load<P>(&self, path: P)
    where
        P: AsRef<Path>,
    {
        tracing::info!("Loading {:?}", path.as_ref());

        let items = JsonLoader::new(path.as_ref());

        let mut map = self.items.write();
        for item in items {
            let id = self.item_id.fetch_add(1, Ordering::Relaxed);
            assert!(id != u32::MAX);

            map.insert(ItemId(WeakId(id)), Arc::new(item));
        }

        tracing::info!("Loaded {:?}", path.as_ref());
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
