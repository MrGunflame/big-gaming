use bevy::prelude::Resource;
use game_net::conn::ConnectionHandle;
use game_net::snapshot::Command;

#[derive(Clone, Debug, Resource)]
pub struct ServerConnection {
    handle: ConnectionHandle,
}

impl ServerConnection {
    pub fn new(handle: ConnectionHandle) -> Self {
        Self { handle }
    }

    pub fn send(&self, cmd: Command) {
        self.handle.send_cmd(cmd);
    }
}
