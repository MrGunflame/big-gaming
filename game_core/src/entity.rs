use game_common::components::components::{Component, Components};
use game_common::components::object::ObjectId;
use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::entity::{Actor, Entity, EntityBody, Object};
use game_common::world::world::WorldViewMut;
use game_data::record::RecordBody;

use crate::modules::Modules;

#[derive(Clone, Debug)]
pub struct SpawnEntity {
    pub id: RecordReference,
    pub transform: Transform,
    pub is_host: bool,
}

impl SpawnEntity {
    pub fn spawn(self, modules: &Modules, view: &mut WorldViewMut<'_>) -> Option<EntityId> {
        let Some(module) = modules.get(self.id.module) else {
            return None;
        };

        let Some(record) = module.records.get(self.id.record) else {
            return None;
        };

        let body = match &record.body {
            RecordBody::Item(item) => todo!(),
            RecordBody::Action(_) => return None,
            RecordBody::Component(_) => return None,
            RecordBody::Object(object) => EntityBody::Object(Object {
                id: ObjectId(self.id),
            }),
            RecordBody::Race(race) => EntityBody::Actor(Actor {
                race: RaceId(self.id),
            }),
        };

        let mut components = Components::new();
        for component in &record.components {
            components.insert(
                component.id,
                Component {
                    bytes: component.bytes.clone(),
                },
            );
        }

        Some(view.spawn(Entity {
            id: EntityId::dangling(),
            transform: self.transform,
            body,
            is_host: self.is_host,
            components,
        }))
    }
}
