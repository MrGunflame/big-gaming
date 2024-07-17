use game_common::module::{Dependency, Module, Version};
use game_data::record::{Record, RecordKind};
use game_data::{DataBuffer, Encode};

use crate::types::Root;

pub fn encode(root: Root) -> Vec<u8> {
    let mut buffer = DataBuffer::new(Module {
        id: root.id.0,
        name: root.name,
        version: Version::new(0, 1, 0),
        dependencies: root
            .dependencies
            .into_iter()
            .map(|dep| Dependency {
                id: dep.id.0,
                name: dep.name,
                version: Version::PLACEHOLDER,
            })
            .collect(),
    });

    for record in root.records {
        buffer.records.push(Record {
            id: record.id.0,
            kind: RecordKind(record.kind.0),
            name: record.name,
            description: record.description,
            data: record.data,
        });
    }

    for script in root.scripts {
        buffer.scripts.push(script);
    }

    let mut buf = Vec::new();
    buffer.encode(&mut buf);
    buf
}
