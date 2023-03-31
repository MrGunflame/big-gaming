use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy::prelude::{Entity, EventWriter, Res, Resource};
use game_common::entity::{EntityId, EntityMap};
use game_common::scene::{Scene, SceneTransition, ServerError};
use game_net::conn::ConnectionHandle;
use game_net::snapshot::{Command, CommandQueue};
use parking_lot::{Mutex, RwLock};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Resource)]
pub struct ServerConnection {
    inner: Arc<ConnectionInner>,
}

#[derive(Debug)]
struct ConnectionInner {
    handle: RwLock<Option<ConnectionHandle>>,
    entities: EntityMap,
    /// State changes
    state: mpsc::Sender<State>,
    state_rx: Mutex<mpsc::Receiver<State>>,
    interpolation_period: RwLock<InterpolationPeriod>,
}

impl ServerConnection {
    pub fn new(map: EntityMap) -> Self {
        let (tx, rx) = mpsc::channel(8);

        let now = Instant::now();

        Self {
            inner: Arc::new(ConnectionInner {
                handle: RwLock::new(None),
                entities: map,
                state: tx,
                state_rx: Mutex::new(rx),
                interpolation_period: RwLock::new(InterpolationPeriod {
                    start: now,
                    end: now,
                }),
            }),
        }
    }

    pub fn send(&self, cmd: Command) {
        let handle = self.inner.handle.read();

        if let Some(handle) = &*handle {
            handle.send_cmd(cmd);
        }
    }

    pub fn lookup(&self, id: Entity) -> Option<EntityId> {
        self.inner.entities.get_entity(id)
    }

    pub fn connect<T>(&self, queue: CommandQueue, addr: T)
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
                *self.inner.handle.write() = Some(handle);
                self.push_state(State::Connecting);
            }
            Err(err) => {
                self.push_state(State::Failed(err.into()));
            }
        }
    }

    pub fn shutdown(&mut self) {
        // The connection will automatically shut down after the last
        // handle was dropped.
        *self.inner.handle.write() = None;
        self.push_state(State::Disconnected);
    }

    pub fn push_state(&self, state: State) {
        let _ = self.inner.state.try_send(state);
    }

    pub fn interpolation_period(&self) -> &RwLock<InterpolationPeriod> {
        &self.inner.interpolation_period
    }
}

pub fn update_connection_state(
    conn: Res<ServerConnection>,
    mut writer: EventWriter<SceneTransition>,
) {
    let mut rx = conn.inner.state_rx.lock();

    while let Ok(state) = rx.try_recv() {
        match state {
            State::Connected => {
                writer.send(SceneTransition {
                    from: Scene::Loading,
                    to: Scene::World,
                });
            }
            State::Connecting => {
                writer.send(SceneTransition {
                    from: Scene::MainMenu,
                    to: Scene::Loading,
                });
            }
            State::Disconnected => {
                writer.send(SceneTransition {
                    from: Scene::World,
                    to: Scene::MainMenu,
                });
            }
            State::Failed(err) => {
                writer.send(SceneTransition {
                    from: Scene::Loading,
                    to: Scene::ServerError(ServerError::Connection(err)),
                });
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum State {
    /// Normally disconnected
    #[default]
    Disconnected,
    /// Currently connecting
    Connecting,
    /// Sucessfully connected
    Connected,
    Failed(Arc<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InterpolationPeriod {
    pub start: Instant,
    pub end: Instant,
}
