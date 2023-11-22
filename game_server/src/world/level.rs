use ahash::{HashMap, HashSet};
use game_common::components::components::{Component, Components};
use game_common::components::items::ItemId;
use game_common::components::object::ObjectId;
use game_common::entity::EntityId;
use game_common::events::{CellLoadEvent, CellUnloadEvent, Event};
use game_common::world::cell::{square, Cell};
use game_common::world::entity::{Entity, EntityBody, Item, Object};
use game_common::world::gen::{CellBuilder, EntityBuilder, Generator};
use game_common::world::CellId;
use game_core::modules::Modules;
use game_data::record::RecordBody;
use glam::Vec3;

use crate::ServerState;

pub fn update_level_cells(state: &mut ServerState) {
    let Some(mut view) = state.world.back_mut() else {
        return;
    };

    let mut cells = HashSet::default();

    for (id, source) in view.streaming_sources().iter() {
        let entity = view.get(id).unwrap();
        let cell = CellId::from(entity.transform.translation);

        let area = square(cell, source.distance);
        cells.extend(area);
    }

    for cell in &cells {
        // If the cell is already loaded, don't update
        // anything.
        if state.level.loaded.contains(cell) {
            state.level.loaded.remove(cell);
            continue;
        }

        if !state.level.cells.contains_key(cell) {
            let mut builder = CellBuilder::new(*cell);
            state.level.generator.generate(&mut builder);

            let mut cell = Cell::new(*cell);

            for entity in builder.into_entities() {
                if let Some(entity) = build_entity(&state.modules, cell.id(), entity) {
                    cell.spawn(entity);
                }
            }

            state.level.cells.insert(cell.id(), cell);
        }

        tracing::info!("loading cell {:?}", cell);

        let cell = state.level.cells.get_mut(cell).unwrap();
        cell.load(&mut view);
        state
            .event_queue
            .push(Event::CellLoad(CellLoadEvent { cell: cell.id() }));
    }

    for cell in &state.level.loaded {
        tracing::info!("unloading cell {:?}", cell);

        let cell = state.level.cells.get_mut(cell).unwrap();
        cell.unload(&mut view);
        state
            .event_queue
            .push(Event::CellUnload(CellUnloadEvent { cell: cell.id() }));
    }

    state.level.loaded = cells;
}

pub struct Level {
    loaded: HashSet<CellId>,
    cells: HashMap<CellId, Cell>,
    generator: Generator,
}

impl Level {
    pub fn new(generator: Generator) -> Self {
        Self {
            loaded: HashSet::default(),
            cells: HashMap::default(),
            generator,
        }
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
