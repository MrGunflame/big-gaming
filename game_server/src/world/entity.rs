use game_common::world::entity::{Entity, EntityBody};
use game_core::modules::Modules;
use game_data::record::RecordBody;
use game_scene::scene2::{Key, Node};

use crate::SceneState;

use super::state::WorldState;

pub fn spawn_entity(
    entity: Entity,
    world: &mut WorldState,
    state: &mut SceneState,
    modules: &Modules,
) -> Key {
    let id = match entity.body {
        EntityBody::Object(id) => id.id.0,
        EntityBody::Actor(id) => id.race.0,
        _ => todo!(),
    };

    let module = modules.get(id.module).unwrap();
    let record = module.records.get(id.record).unwrap();
    let obj = match &record.body {
        RecordBody::Object(obj) => &obj.uri,
        RecordBody::Race(r) => &r.model,
        _ => todo!(),
    };

    let key = state.graph.append(
        None,
        Node {
            transform: entity.transform,
            components: vec![],
        },
    );

    state.spawner.spawn(key, obj);
    key
}
