use std::collections::HashMap;

use game_common::components::components::RawComponent;
use game_common::components::{BinaryReader, Children, Component, Decode};
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::World;
use game_wasm::encoding::{decode_fields, encode_fields, BinaryWriter};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneFile {
    // EntityId => [RecordReference => RawComponent]
    entities: HashMap<u64, HashMap<String, EncodedComponent>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EncodedComponent {
    bytes: Vec<u8>,
    fields: Vec<u8>,
}

impl SceneFile {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: EntityId, entity: &World) {
        let mut components = HashMap::new();
        for (id, component) in entity.components(id).iter() {
            components.insert(
                id.to_string(),
                EncodedComponent {
                    bytes: component.as_bytes().to_vec(),
                    fields: encode_fields(component.fields()),
                },
            );
        }

        let index = self.entities.len() as u64;
        self.entities.insert(index, components);
    }

    pub fn instantiate(self, world: &mut World) {
        let mut entities = HashMap::new();

        for (entity, components) in self.entities {
            let entity_id = world.spawn();
            entities.insert(entity, entity_id);

            for (id, component) in components {
                let id: RecordReference = id.parse().unwrap();

                let fields = decode_fields(&component.fields);
                let component = RawComponent::new(component.bytes, fields);

                if id == Children::ID {
                    let reader = BinaryReader::new(
                        component.as_bytes().to_vec(),
                        component.fields().to_vec().into(),
                    );
                    let children = Children::decode(reader).unwrap();

                    let mut new_children = Children::new();
                    for id in children.get() {
                        let id = id.into_raw();
                        let child = entities.get(&id).unwrap();
                        new_children
                            .insert(game_wasm::entity::EntityId::from_raw(child.into_raw()));
                    }

                    let (fields, bytes) = BinaryWriter::new().encoded(&new_children);
                    let component = RawComponent::new(bytes, fields);
                    world.insert(entity_id, id, component);
                    continue;
                }

                world.insert(entity_id, id, component);
            }
        }
    }
}
