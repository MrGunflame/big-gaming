use game_common::module::{Dependency, Module, Version};
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

    let mut buf = Vec::new();
    buffer.encode(&mut buf);
    buf
}
