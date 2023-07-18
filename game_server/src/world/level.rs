use ahash::{HashMap, HashSet};
use bevy_ecs::system::Resource;
use game_common::components::components::{Component, Components};
use game_common::components::items::ItemId;
use game_common::components::object::ObjectId;
use game_common::entity::EntityId;
use game_common::events::{CellLoadEvent, CellUnloadEvent, Event, EventQueue};
use game_common::world::cell::{square, Cell};
use game_common::world::entity::{Entity, EntityBody, Item, Object};
use game_common::world::gen::{CellBuilder, EntityBuilder, Generator};
use game_common::world::world::WorldState;
use game_common::world::CellId;
use game_core::modules::Modules;
use game_data::record::RecordBody;

pub fn update_level_cells(
    world: &mut WorldState,
    level: &mut Level,
    modules: &Modules,
    events: &mut EventQueue,
) {
    let Some(mut view) = world.back_mut() else {
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
        if level.loaded.contains(cell) {
            level.loaded.remove(cell);
            continue;
        }

        if !level.cells.contains_key(cell) {
            let mut builder = CellBuilder::new(*cell);
            level.generator.generate(&mut builder);

            let mut cell = Cell::new(*cell);

            for entity in builder.into_entities() {
                if let Some(entity) = build_entity(modules, cell.id(), entity) {
                    cell.spawn(entity);
                }
            }

            level.cells.insert(cell.id(), cell);
        }

        tracing::info!("loading cell {:?}", cell);

        let cell = level.cells.get_mut(cell).unwrap();
        cell.load(&mut view);
        events.push(Event::CellLoad(CellLoadEvent { cell: cell.id() }));
    }

    for cell in &level.loaded {
        tracing::info!("unloading cell {:?}", cell);

        let cell = level.cells.get_mut(cell).unwrap();
        cell.unload(&mut view);
        events.push(Event::CellUnload(CellUnloadEvent { cell: cell.id() }));
    }

    level.loaded = cells;
}

#[derive(Resource)]
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
        });
    }

    let Some(module) = modules.get(builder.id.module) else {
        tracing::error!("load error: unknown module {:?} in {:?}", builder.id.module, builder.id);
        return None;
    };

    let Some(record) = module.records.get(builder.id.record) else {
        tracing::error!("load error: unknown record {:?} in {:?}", builder.id.record, builder.id);
        return None;
    };

    let mut components = Components::new();

    let body = match &record.body {
        RecordBody::Item(item) => {
            for component in &item.components {
                components.insert(
                    component.record,
                    Component {
                        bytes: component.value.clone(),
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
    };

    Some(Entity {
        id: EntityId::dangling(),
        transform: builder.transform,
        components,
        body,
    })
}
