use ahash::HashMap;
use game_common::components::inventory::Inventory;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_common::world::entity::Entity;
use game_script::effect::Effect;
use game_script::executor::ScriptExecutor;
use game_script::{Context, WorldProvider};

use crate::net::world::{Command, CommandBuffer};

use super::state::WorldState;

pub fn run_scripts(
    world: &mut WorldState,
    physics_pipeline: &game_physics::Pipeline,
    executor: &ScriptExecutor,
    event_queue: &mut EventQueue,
    buffer: &mut CommandBuffer,
) {
    let effects = executor.run(Context {
        view: world,
        physics_pipeline,
        events: event_queue,
    });

    // Since the script executing uses its own temporary ID namespace
    // for newly created IDs we must remap all IDs into "real" IDs.
    // A temporary ID must **never** overlap with an existing ID.
    // FIXME: We should use a linear IDs here so we can avoid
    // the need for hasing and just use array indexing.
    let mut inventory_slot_id_remap = HashMap::default();

    for effect in effects.into_iter() {
        match effect {
            Effect::EntitySpawn(entity) => {
                todo!()
            }
            Effect::EntityDespawn(id) => todo!(),
            Effect::EntityTranslate(id, translation) => {
                buffer.push(Command::Translate {
                    entity: id,
                    dst: translation,
                });

                world.entities.get_mut(id).unwrap().transform.translation = translation;
            }
            Effect::EntityRotate(id, rotation) => {
                buffer.push(Command::Rotate {
                    entity: id,
                    dst: rotation,
                });

                world.entities.get_mut(id).unwrap().transform.rotation = rotation;
            }
            Effect::InventoryInsert(id, temp_slot_id, stack) => {
                if world.inventories.get(id).is_none() {
                    world.inventories.insert(id);
                }

                let inventory = world.inventories.get_mut(id).unwrap();
                let real_slot_id = inventory.insert(stack).unwrap();

                inventory_slot_id_remap.insert(temp_slot_id, real_slot_id);
            }
            Effect::InventoryRemove(id, slot_id, quantity) => {
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory.remove(slot_id, quantity as u32);
            }
            Effect::InventoryItemUpdateEquip(id, slot_id, equipped) => {
                let slot_id = inventory_slot_id_remap
                    .get(&slot_id)
                    .copied()
                    .unwrap_or(slot_id);

                let inventory = world.inventories.get_mut(id).unwrap();
                inventory.get_mut(slot_id).unwrap().item.equipped = equipped;
            }
            _ => todo!(),
        }
    }
}

impl WorldProvider for WorldState {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(id)
    }

    fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(id)
    }
}
