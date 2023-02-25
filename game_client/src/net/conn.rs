use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use bevy::prelude::{Entity, EventWriter, Res, ResMut, Resource};
use game_common::entity::{EntityId, EntityMap};
use game_common::scene::{Scene, SceneTransition, ServerError};
use game_net::conn::ConnectionHandle;
use game_net::snapshot::{Command, CommandQueue};

#[derive(Clone, Debug, Resource)]
pub struct ServerConnection {
    handle: Option<ConnectionHandle>,
    map: EntityMap,
    state: State,
    // Last state published to scene event readers.
    is_published: bool,
}

impl ServerConnection {
    pub fn new(handle: ConnectionHandle, map: EntityMap) -> Self {
        Self {
            handle: Some(handle),
            map,
            state: State::Active,
            is_published: true,
        }
    }

    pub fn stub(map: EntityMap) -> Self {
        Self {
            handle: None,
            map,
            state: State::Idle,
            is_published: true,
        }
    }

    pub fn send(&self, cmd: Command) {
        if let Some(handle) = &self.handle {
            handle.send_cmd(cmd);
        }
    }

    pub fn lookup(&self, id: Entity) -> Option<EntityId> {
        self.map.get_entity(id)
    }

    pub fn connect<T>(&mut self, queue: CommandQueue, addr: T)
    where
        T: ToSocketAddrs,
    {
        fn inner(
            queue: CommandQueue,
            addr: impl ToSocketAddrs,
        ) -> Result<ConnectionHandle, Box<dyn std::error::Error + Send + Sync + 'static>> {
            // TODO: Use async API
            let addr = match addr.to_socket_addrs()?.nth(0) {
                Some(addr) => addr,
                None => panic!("empty dns result"),
            };

            super::spawn_conn(queue, addr)
        }

        match inner(queue, addr) {
            Ok(handle) => {
                self.handle = Some(handle);
                self.state = State::Active;
            }
            Err(err) => {
                self.state = State::Failed(err.into());
            }
        }

        self.is_published = false;
    }

    pub fn shutdown(&mut self) {
        // The connection will automatically shut down after the last
        // handle was dropped.
        self.handle = None;
        self.state = State::Idle;

        self.is_published = false;
    }
}

pub fn update_connection_state(
    mut conn: ResMut<ServerConnection>,
    mut writer: EventWriter<SceneTransition>,
) {
    if !conn.is_published {
        conn.is_published = true;

        match &conn.state {
            State::Active => {
                writer.send(SceneTransition {
                    from: Scene::Loading,
                    to: Scene::World,
                });
            }
            State::Idle => {
                writer.send(SceneTransition {
                    from: Scene::World,
                    to: Scene::MainMenu,
                });
            }
            State::Failed(err) => {
                writer.send(SceneTransition {
                    from: Scene::Loading,
                    to: Scene::ServerError(ServerError::Connection(err.clone())),
                });
            }
        }
    }
}

#[derive(Clone, Debug)]
enum State {
    Idle,
    Active,
    Failed(Arc<dyn std::error::Error + Send + Sync + 'static>),
}
