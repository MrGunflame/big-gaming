mod items;

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use bevy_ecs::system::Resource;

use self::items::Item;
use crate::components::items::ItemId;

/// The entrypoint of loading external data.
#[derive(Debug, Resource)]
pub struct GameArchive {
    items: HashMap<ItemId, Arc<Item>>,
}

impl GameArchive {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn item(&self, id: ItemId) -> Option<Ref<'_, Item>> {
        self.items.get(&id).map(|item| Ref {
            archive: self,
            item: item.clone(),
        })
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
