use ahash::HashMap;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_core::modules::Modules;
use game_script::effect::Effect;
use game_script::{Context, Executor, WorldProvider};

use super::state::WorldState;

pub fn run_scripts(
    world: &mut WorldState,
    physics_pipeline: &game_physics::Pipeline,
    executor: &mut Executor,
    event_queue: &mut EventQueue,
    modules: &Modules,
) {
    let effects = executor.update(Context {
        world,
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
                world.world.despawn(id);
            }
            Effect::EntityComponentInsert(effect) => {
                let entity = entity_id_remap
                    .get(&effect.entity)
                    .copied()
                    .unwrap_or(effect.entity);

                world
                    .world
                    .insert(entity, effect.component_id, effect.component);
            }
            Effect::EntityComponentRemove(effect) => {
                let entity = entity_id_remap
                    .get(&effect.entity)
                    .copied()
                    .unwrap_or(effect.entity);

                world.world.remove(entity, effect.component_id);
            }
            Effect::PlayerSetActive(_) => (),
        }
    }
}

impl WorldProvider for WorldState {
    fn world(&self) -> &game_common::world::World {
        &self.world
    }

    fn player(&self, id: EntityId) -> Option<game_wasm::player::PlayerId> {
        // TODO: We only know about our own player id and should return
        // it if appropriate.
        None
    }
}
