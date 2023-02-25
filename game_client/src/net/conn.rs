use std::net::SocketAddr;

use bevy::prelude::{Entity, Resource};
use game_common::entity::{EntityId, EntityMap};
use game_net::conn::ConnectionHandle;
use game_net::snapshot::{Command, CommandQueue};

#[derive(Clone, Debug, Resource)]
pub struct ServerConnection {
    handle: Option<ConnectionHandle>,
    map: EntityMap,
}

impl ServerConnection {
    pub fn new(handle: ConnectionHandle, map: EntityMap) -> Self {
        Self {
            handle: Some(handle),
            map,
        }
    }

    pub fn stub(map: EntityMap) -> Self {
        Self { handle: None, map }
    }

    pub fn send(&self, cmd: Command) {
        if let Some(handle) = &self.handle {
            handle.send_cmd(cmd);
        }
    }

    pub fn lookup(&self, id: Entity) -> Option<EntityId> {
        self.map.get_entity(id)
    }

    pub fn connect(&mut self, queue: CommandQueue, addr: SocketAddr) {
        let handle = super::spawn_conn(queue, addr);
        self.handle = Some(handle);
    }

    pub fn shutdown(&mut self) {
        // The connection will automatically shut down after the last
        // handle was dropped.
        self.handle = None;
    }
}
