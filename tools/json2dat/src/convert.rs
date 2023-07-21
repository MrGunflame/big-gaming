use std::path::PathBuf;

use game_common::module::{Dependency, Module, Version};
use game_data::components::actions::ActionRecord;
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
        };

        buffer.records.push(record);
    }

    buffer.header.records = buffer.records.len() as u32;

    let mut buf = Vec::new();
    buffer.encode(&mut buf);
    buf
}
