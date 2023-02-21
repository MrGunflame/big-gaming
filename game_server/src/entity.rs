use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::Resource;
use game_common::net::ServerEntity;

#[derive(Debug, Resource)]
pub struct ServerEntityGenerator {
    id: AtomicU64,
}

impl ServerEntityGenerator {
    pub fn new() -> Self {
        Self {
            id: AtomicU64::new(0),
        }
    }

    pub fn generate(&self) -> ServerEntity {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        ServerEntity(id)
    }
}
