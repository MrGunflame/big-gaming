use game_common::components::components::RawComponent;
use game_common::components::inventory::InventorySlotId;
use game_common::components::items::ItemStack;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_wasm::player::PlayerId;

#[derive(Clone, Debug, Default)]
pub struct Effects {
    effects: Vec<Effect>,
}

impl Effects {
    pub fn push(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn into_iter(self) -> impl Iterator<Item = Effect> {
        self.effects.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ Effect> + '_ {
        self.effects.iter()
    }
}

#[derive(Clone, Debug)]
pub enum Effect {
    EntitySpawn(EntityId),
    EntityDespawn(EntityId),
    EntityComponentInsert(EntityId, RecordReference, Vec<u8>),
    EntityComponentRemove(EntityId, RecordReference),
    InventoryInsert(EntityId, InventorySlotId, ItemStack),
    InventoryRemove(EntityId, InventorySlotId, u64),
    InventoryClear(EntityId),
    InventoryComponentInsert(EntityId, InventorySlotId, RecordReference, RawComponent),
    InventoryComponentRemove(EntityId, InventorySlotId, RecordReference),
    InventoryItemUpdateEquip(EntityId, InventorySlotId, bool),
    PlayerSetActive(PlayerSetActive),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSetActive {
    pub player: PlayerId,
    pub entity: EntityId,
}
