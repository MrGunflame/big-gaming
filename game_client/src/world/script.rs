use ahash::HashMap;
use game_common::components::components::Component;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_core::modules::Modules;
use game_script::effect::Effect;
use game_script::{Context, Executor, WorldProvider};

use crate::net::world::CommandBuffer;

use super::state::WorldState;

pub fn run_scripts(
    world: &mut WorldState,
    physics_pipeline: &game_physics::Pipeline,
    executor: &mut Executor,
    event_queue: &mut EventQueue,
    buffer: &mut CommandBuffer,
    modules: &Modules,
) {
    let effects = executor.update(Context {
        world: world,
        physics: physics_pipeline,
        events: event_queue,
        records: modules,
    });

    // Since the script executing uses its own temporary ID namespace
    // for newly created IDs we must remap all IDs into "real" IDs.
    // A temporary ID must **never** overlap with an existing ID.
    // FIXME: We should use a linear IDs here so we can avoid
    // the need for hasing and just use array indexing.
    let mut entity_id_remap = HashMap::default();
    let mut inventory_slot_id_remap = HashMap::default();

    for effect in effects.into_iter() {
        match effect {
            Effect::EntitySpawn(entity) => {
                debug_assert!(entity_id_remap.get(&entity).is_none());

                let temp_id = entity;
                let real_id = world.world.spawn();
                entity_id_remap.insert(temp_id, real_id);
            }
            Effect::EntityDespawn(id) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);
                world.inventories.remove(id);
                world.world.despawn(id);
            }
            Effect::InventoryInsert(id, temp_slot_id, stack) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);

                if world.inventories.get(id).is_none() {
                    world.inventories.insert(id);
                }

                let inventory = world.inventories.get_mut(id).unwrap();
                let real_slot_id = inventory.insert(stack).unwrap();

                inventory_slot_id_remap.insert(temp_slot_id, real_slot_id);
            }
            Effect::InventoryRemove(id, slot_id, quantity) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory.remove(slot_id, quantity as u32);
            }
            Effect::InventoryItemUpdateEquip(id, slot_id, equipped) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory.get_mut(slot_id).unwrap().item.equipped = equipped;
            }
            Effect::InventoryComponentInsert(id, slot_id, component, data) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory
                    .get_mut(slot_id)
                    .unwrap()
                    .item
                    .components
                    .insert(component, data);
            }
            Effect::InventoryComponentRemove(id, slot_id, component) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory
                    .get_mut(slot_id)
                    .unwrap()
                    .item
                    .components
                    .remove(component);
            }
            Effect::InventoryClear(id) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory.clear();
            }
            Effect::EntityComponentInsert(id, component, data) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);

                world.world.insert(id, component, Component::new(data));
            }
            Effect::EntityComponentRemove(id, component) => {
                let id = entity_id_remap.get(&id).copied().unwrap_or(id);

                world.world.remove(id, component);
            }
        }
    }
}

impl WorldProvider for WorldState {
    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(id)
    }

    fn world(&self) -> &game_common::world::World {
        &self.world
    }
}
