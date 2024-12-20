use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use game_wasm::cell::CellId;
use game_wasm::components::builtin::{
    Collider, ColliderShape, Color, Cuboid, DirectionalLight, Global, MeshInstance, RigidBody,
    RigidBodyKind, Transform,
};
use game_wasm::entity::EntityId;
use game_wasm::events::CellLoad;
use game_wasm::math::Vec3;
use game_wasm::prefab::spawn_prefab;
use game_wasm::record::{get_record_list, ModuleId, Record, RecordFilter, RecordId};
use game_wasm::resource::ResourceId;
use game_wasm::world::{Entity, RecordReference};
use game_worldgen::WorldgenState;

use crate::assets;
use crate::weather::{sun_rotation, DateTime};

pub fn cell_load(_: EntityId, event: CellLoad) {
    let min = event.cell.min();
    let max = event.cell.max();

    if min.y != 0.0 {
        return;
    }

    let mut translation = min + (max - min) * 0.5;
    translation.y = min.y;

    let entity = Entity::spawn();
    entity.insert(Transform::from_translation(translation));
    entity.insert(RigidBody {
        kind: RigidBodyKind::Fixed,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    });
    entity.insert(Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::Cuboid(Cuboid {
            hx: (max.x - min.x) / 2.0,
            hy: 0.1,
            hz: (max.z - min.z) / 2.0,
        }),
    });
    entity.insert(MeshInstance {
        model: ResourceId::from(assets::RESOURCE_FLOOR),
    });

    if let Ok(false) =
        WEATHER_INIT.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
    {
        init_weather(min);
    }

    static GENERATED_CELLS: Mutex<Vec<CellId>> = Mutex::new(Vec::new());
    let mut generated_cells = GENERATED_CELLS.lock().unwrap();
    if !generated_cells.contains(&event.cell) {
        generated_cells.push(event.cell);
        generate_cell(event.cell);
    }
}

static WEATHER_INIT: AtomicBool = AtomicBool::new(false);

fn init_weather(min: Vec3) {
    let sun = Entity::spawn();
    sun.insert(Transform {
        translation: min,
        rotation: sun_rotation(DateTime::from_secs(60 * 60 * 12)),
        ..Default::default()
    });
    sun.insert(DirectionalLight {
        color: Color::WHITE,
        illuminance: 100_000.0,
    });
    sun.insert(Global);
}

fn generate_cell(cell: CellId) {
    static GENERATOR: OnceLock<WorldgenState> = OnceLock::new();

    let generator = GENERATOR.get_or_init(|| {
        const WORLD_GEN: RecordReference = RecordReference {
            module: ModuleId::CORE,
            record: RecordId(0x1006),
        };

        let records = get_record_list(&RecordFilter {
            module: None,
            kind: Some(WORLD_GEN),
        });

        let mut state = WorldgenState::new();

        for id in records {
            let record = Record::get(id).unwrap();

            match WorldgenState::from_bytes(record.data()) {
                Ok(s) => {
                    state.extend(s);
                }
                Err(err) => {
                    game_wasm::error!("record {} contains invalid world gen data: {:?}", id, err);
                }
            }
        }

        state
    });

    for data in generator.load(cell) {
        match spawn_prefab(data.prefab) {
            Ok(entity) => {
                Entity::new(entity).insert(data.transform);
            }
            Err(err) => {
                game_wasm::error!("failed to spawn prefab: {}", err);
            }
        }
    }
}
