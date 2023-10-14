use std::path::PathBuf;

use game_common::module::{Dependency, Module, Version};
use game_data::components::actions::ActionRecord;
use game_data::components::components::{ComponentRecord, ComponentValue};
use game_data::components::objects::ObjectRecord;
use game_data::components::race::RaceRecord;
use game_data::record::{Record, RecordBody};
use game_data::uri::Uri;
use game_data::{DataBuffer, Encode};

use crate::types::Root;

pub fn encode(root: Root) -> Vec<u8> {
    let mut buffer = DataBuffer::new(Module {
        id: root.id.0,
        name: root.name,
        version: Version,
        dependencies: root
            .dependencies
            .into_iter()
            .map(|dep| Dependency {
                id: dep.id.0,
                name: dep.name,
                version: Version,
            })
            .collect(),
    });

    for action in root.records.actions {
        let record = Record {
            id: action.id.0,
            name: action.name,
            scripts: action
                .scripts
                .into_iter()
                .map(|s| Uri::from(PathBuf::from(s)))
                .collect(),
            body: RecordBody::Action(ActionRecord {
                description: action.description,
            }),
            components: vec![],
        };

        buffer.records.push(record);
    }

    for race in root.records.races {
        let components = race
            .components
            .into_iter()
            .map(|(id, val)| ComponentValue {
                id: id.0,
                bytes: val,
            })
            .collect();

        let record = Record {
            id: race.id.0,
            name: race.name,
            scripts: vec![],
            body: RecordBody::Race(RaceRecord {
                model: Uri::from(PathBuf::from(race.model)),
                actions: race.actions.into_iter().map(|a| a.0).collect(),
            }),
            components,
        };

        buffer.records.push(record);
    }

    for component in root.records.components {
        let record = Record {
            id: component.id.0,
            name: component.name,
            scripts: component
                .scripts
                .into_iter()
                .map(|s| Uri::from(PathBuf::from(s)))
                .collect(),
            body: RecordBody::Component(ComponentRecord {
                description: component.description,
            }),
            components: vec![],
        };

        buffer.records.push(record);
    }

    for object in root.records.objects {
        let record = Record {
            id: object.id.0,
            name: object.name,
            scripts: object
                .scripts
                .into_iter()
                .map(|s| Uri::from(PathBuf::from(s)))
                .collect(),
            body: RecordBody::Object(ObjectRecord {
                uri: Uri::from(PathBuf::from(object.model)),
                components: vec![],
            }),
            components: vec![],
        };

        buffer.records.push(record);
    }

    buffer.header.records = buffer.records.len() as u32;

    let mut buf = Vec::new();
    buffer.encode(&mut buf);
    buf
}
