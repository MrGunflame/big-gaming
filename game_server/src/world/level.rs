use std::sync::Arc;

use ahash::HashMap;
use bevy_ecs::system::Resource;
use game_common::components::transform::PreviousTransform;
use game_common::world::cell::Cell;
use game_common::world::gen::flat::FlatGenerator;
use game_common::world::gen::Generator;
use game_common::world::snapshot::EntityChange;
use game_common::world::source::{StreamingSource, StreamingSources, StreamingState};
use game_common::world::world::WorldState;
use game_common::world::CellId;
use parking_lot::RwLock;

pub fn update_streaming_sources(mut sources: &mut StreamingSources, world: &WorldState) {
    let Some(view) = world.back() else {
        return;
    };

    sources.clear();

    let mut load = vec![];
    let mut unload = vec![];

    for event in view.deltas() {
        match event {
            EntityChange::UpdateStreamingSource { id, state } => {
                let entity = view.get(*id).unwrap();
                let cell = CellId::from(entity.transform.translation);

                match state {
                    StreamingState::Create => {
                        load.push(cell);
                    }
                    StreamingState::Destroy => {
                        unload.push(cell);
                    }
                    _ => (),
                }
            }
            EntityChange::Translate {
                id,
                translation: _,
                cell,
            } => {
                if let Some(cell) = cell {
                    if view.streaming_sources().get(*id).is_some() {
                        load.push(cell.to);
                        // unload.push(cell.from);
                    }
                }
            }
            _ => (),
        }
    }

    load.dedup();
    unload.dedup();

    for id in load {
        sources.load(id);
    }

    for id in unload {
        sources.unload(id);
    }

    // for (transform, prev, mut source) in &mut entities {
    //     let new_id = CellId::from(transform.translation);
    //     let prev_id = CellId::from(prev.translation);

    //     if source.state.is_active() && new_id == prev_id {
    //         continue;
    //     }

    //     let mut load = Vec::with_capacity(32);
    //     let mut unload = Vec::with_capacity(32);

    //     // match source.state {
    //     //     StreamingSource::Create => {
    //     //         load.push(new_id);
    //     //     }
    //     // }
    // }
}

pub fn update_level(sources: &StreamingSources, level: &Level, mut world: &mut WorldState) {
    let Some(mut view) = world.back_mut() else {
        return;
    };

    for id in sources.loaded() {
        tracing::info!("loading cell {:?}", id);

        let cell = level.get(id);
        let mut cell = cell.write();

        cell.load(&mut view);
    }

    for id in sources.unloaded() {
        tracing::info!("unloading cell {:?}", id);

        let cell = level.get(id);
        let mut cell = cell.write();

        cell.unload(&mut view);
    }

    drop(view);
    let view = world.back().unwrap();

    for id in sources.iter() {
        let view = view.cell(id);

        let cell = level.get(id);
        let mut cell = cell.write();

        cell.update(&view);
    }
}

#[derive(Resource)]
pub struct Level {
    cells: RwLock<HashMap<CellId, Arc<RwLock<Cell>>>>,
    generator: Generator,
}

impl Level {
    pub fn new() -> Self {
        Self {
            cells: RwLock::default(),
            generator: Generator::from(FlatGenerator),
        }
    }

    pub fn get(&self, id: CellId) -> Arc<RwLock<Cell>> {
        let cells = self.cells.read();
        if let Some(cell) = cells.get(&id) {
            return cell.clone();
        }

        drop(cells);
        let mut cells = self.cells.write();

        let mut cell = Cell::new(id);
        self.generator.generate(&mut cell);

        let cell = Arc::new(RwLock::new(cell));
        cells.insert(id, cell.clone());
        cell
    }
}
