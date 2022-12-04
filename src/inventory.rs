use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u64);

#[derive(Clone, Debug)]
pub struct Inventory {
    items: HashMap<ItemId, Item>,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.get(&id)
    }

    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        self.items.get_mut(&id)
    }

    /// Inserts a new [`Item`] into the inventory.
    pub fn insert(&mut self, item: Item) {
        match self.get_mut(item.id) {
            Some(item) => item.quantity += 1,
            None => {
                self.items.insert(item.id, item);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub quantity: u32,
}
