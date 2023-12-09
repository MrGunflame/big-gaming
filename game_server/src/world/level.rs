use ahash::{HashMap, HashSet};
use game_common::components::components::{Component, Components};
use game_common::components::items::ItemId;
use game_common::components::object::ObjectId;
use game_common::entity::EntityId;
use game_common::events::{CellLoadEvent, CellUnloadEvent, Event};
use game_common::world::cell::square;
use game_common::world::entity::{Entity, EntityBody, Item, Object};
use game_common::world::gen::{CellBuilder, EntityBuilder, Generator};
use game_common::world::CellId;
use game_core::modules::Modules;
use game_data::record::RecordBody;
use game_script::WorldProvider;
use glam::Vec3;

use crate::world::entity::spawn_entity;
use crate::ServerState;

#[derive(Copy, Clone, Debug)]
pub struct Streamer {
    pub distance: u32,
}

pub fn update_level_cells(state: &mut ServerState) {
    let mut cells = HashSet::default();
    for (entity, streamer) in &state.level.streamers {
        let entity = state.world.get(*entity).unwrap();
        let cell = CellId::from(entity.transform.translation);

        let area = square(cell, streamer.distance);
        cells.extend(area);
    }

    for cell in &cells {
        // If the cell is already loaded, don't update
        // anything.
        if state.level.loaded.contains(cell) {
            state.level.loaded.remove(cell);
            continue;
        }

        if !state.level.loaded.contains(cell) {
            let mut builder = CellBuilder::new(*cell);
            state.level.generator.generate(&mut builder);

            for entity in builder.into_entities() {
                if let Some(entity) = build_entity(&state.modules, *cell, entity) {
                    let key = spawn_entity(
                        entity.clone(),
                        &mut state.world,
                        &mut state.scene,
                        &state.modules,
                    );
                    let id = state.world.insert(entity);

                    state.scene.entities.insert(key, id);
                }
            }

            state.level.loaded.insert(*cell);
        }

        tracing::info!("loading cell {:?}", cell);

        state
            .event_queue
            .push(Event::CellLoad(CellLoadEvent { cell: *cell }));
    }

    for cell in &state.level.loaded {
        // TODO: Unload cell
        state
            .event_queue
            .push(Event::CellUnload(CellUnloadEvent { cell: *cell }));
    }

    state.level.loaded = cells;
}

pub struct Level {
    loaded: HashSet<CellId>,
    streamers: HashMap<EntityId, Streamer>,
    generator: Generator,
}

impl Level {
    pub fn new(generator: Generator) -> Self {
        Self {
            loaded: HashSet::default(),
            streamers: HashMap::default(),
            generator,
        }
    }

    pub fn create_streamer(&mut self, id: EntityId, streamer: Streamer) {
        self.streamers.insert(id, streamer);
    }

    pub fn destroy_streamer(&mut self, id: EntityId) {
        self.streamers.remove(&id);
    }
}

fn build_entity(modules: &Modules, cell: CellId, builder: EntityBuilder) -> Option<Entity> {
    debug_assert!(builder.transform.is_valid());

    if let Some(terrain) = builder.terrain {
        return Some(Entity {
            id: EntityId::dangling(),
            transform: builder.transform,
            body: EntityBody::Terrain(terrain),
            components: Components::new(),
            is_host: false,
            angvel: Vec3::ZERO,
            linvel: Vec3::ZERO,
        });
    }

    let Some(module) = modules.get(builder.id.module) else {
        tracing::error!(
            "load error: unknown module {} in {}",
            builder.id.module,
            builder.id
        );
        return None;
    };

    let Some(record) = module.records.get(builder.id.record) else {
        tracing::error!(
            "load error: unknown record {} in {}",
            builder.id.record,
            builder.id
        );
        return None;
    };

    let mut components = Components::new();

    let body = match &record.body {
        RecordBody::Item(item) => {
            for component in &record.components {
                components.insert(
                    component.id,
                    Component {
                        bytes: component.bytes.clone(),
                    },
                );
            }

            EntityBody::Item(Item {
                id: ItemId(builder.id),
            })
        }
        RecordBody::Action(_) => {
            tracing::error!(
                "load error: attempted to load an action record {} ({:?})",
                record.name,
                record.id,
            );

            return None;
        }
        RecordBody::Component(_) => {
            tracing::error!(
                "load error: attempted to load an component record {} ({:?})",
                record.name,
                record.id
            );

            return None;
        }
        RecordBody::Object(object) => {
            for component in &object.components {
                components.insert(
                    component.record,
                    Component {
                        bytes: component.value.clone(),
                    },
                );
            }

            EntityBody::Object(Object {
                id: ObjectId(builder.id),
            })
        }
        RecordBody::Race(_) => {
            tracing::error!(
                "load error: attempted to load a race record {} ({:?})",
                record.name,
                record.id
            );

            return None;
        }
    };

    Some(Entity {
        id: EntityId::dangling(),
        transform: builder.transform,
        components,
        body,
        is_host: false,
        angvel: builder.angvel,
        linvel: builder.linvel,
    })
}
