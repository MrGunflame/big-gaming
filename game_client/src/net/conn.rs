use bevy::prelude::{Entity, Resource};
use game_common::entity::{EntityId, EntityMap};
use game_net::conn::ConnectionHandle;
use game_net::snapshot::Command;

#[derive(Clone, Debug, Resource)]
pub struct ServerConnection {
    handle: ConnectionHandle,
    map: EntityMap,
}

impl ServerConnection {
    pub fn new(handle: ConnectionHandle, map: EntityMap) -> Self {
        Self { handle, map }
    }

    pub fn send(&self, cmd: Command) {
        self.handle.send_cmd(cmd);
    }

    pub fn lookup(&self, id: Entity) -> Option<EntityId> {
        self.map.get_entity(id)
    }
}
