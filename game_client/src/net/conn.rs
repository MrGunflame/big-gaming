use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{ResMut, Resource};
use bevy_ecs::world::{FromWorld, World};
use game_common::entity::{EntityId, EntityMap};
use game_common::world::control_frame::ControlFrame;
use game_core::time::Time;
use game_net::conn::{ConnectionHandle, ConnectionId};
use game_net::snapshot::{Command, CommandQueue, ConnectionMessage};

use crate::state::{GameState, GameStateWriter};

use super::prediction::{LocalOverrides, Prediction};

#[derive(Debug, Resource)]
pub struct ServerConnection {
    pub handle: Option<ConnectionHandle>,
    pub entities: EntityMap,
    pub overrides: LocalOverrides,
    pub host: EntityId,
    pub interpolation_period: InterpolationPeriod,
    pub writer: GameStateWriter,
    pub queue: CommandQueue,

    game_tick: GameTick,
}

impl ServerConnection {
    pub fn new(writer: GameStateWriter) -> Self {
        Self {
            handle: None,
            entities: EntityMap::default(),
            interpolation_period: InterpolationPeriod::new(),
            overrides: LocalOverrides::new(),
            host: EntityId::dangling(),
            writer,
            queue: CommandQueue::new(),
            game_tick: GameTick {
                current_control_frame: ControlFrame(0),
                last_update: Instant::now(),
            },
        }
    }

    pub fn send(&mut self, cmd: Command) {
        if let Some(handle) = &self.handle {
            let cmd_id = handle.send_cmd(ConnectionMessage {
                id: None,
                conn: ConnectionId(0),
                control_frame: self.game_tick.current_control_frame,
                command: cmd.clone(),
            });

            match cmd {
                Command::EntityTranslate { id, translation } => {
                    let ov = &mut self.overrides;
                    ov.push(id, cmd_id, Prediction::Translation(translation));
                }
                Command::EntityRotate { id, rotation } => {
                    let ov = &mut self.overrides;
                    ov.push(id, cmd_id, Prediction::Rotation(rotation));
                }
                _ => (),
            }
        } else {
            tracing::warn!("attempted to send a command, but the peer is not connected");
        }
    }

    pub fn lookup(&self, id: Entity) -> Option<EntityId> {
        self.entities.get_entity(id)
    }

    pub fn connect<T>(&mut self, addr: T)
    where
        T: ToSocketAddrs,
    {
        self.reset_queue();

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

        match inner(self.queue.clone(), addr) {
            Ok(handle) => {
                self.handle = Some(handle);
                self.writer.update(GameState::Connecting);
            }
            Err(err) => {
                tracing::error!("failed to connect: {}", err);

                self.writer.update(GameState::ConnectionFailure);
            }
        }
    }

    pub fn shutdown(&mut self) {
        // The connection will automatically shut down after the last
        // handle was dropped.
        self.handle = None;
        self.reset_queue();

        self.writer.update(GameState::MainMenu);
    }

    /// Returns the current control frame.
    pub fn control_fame(&self) -> ControlFrame {
        self.game_tick.current_control_frame
    }

    fn reset_queue(&mut self) {
        self.queue = CommandQueue::new();
    }
}

impl FromWorld for ServerConnection {
    fn from_world(world: &mut World) -> Self {
        let writer = world.resource::<GameStateWriter>().clone();
        Self::new(writer)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InterpolationPeriod {
    pub start: ControlFrame,
    pub end: ControlFrame,
}

impl InterpolationPeriod {
    fn new() -> Self {
        Self {
            start: ControlFrame(0),
            end: ControlFrame(0),
        }
    }
}

#[derive(Debug)]
struct GameTick {
    current_control_frame: ControlFrame,
    last_update: Instant,
}

pub fn tick_game(time: ResMut<Time>, mut conn: ResMut<ServerConnection>) {
    if time.last_update() - conn.game_tick.last_update >= Duration::from_secs(1) / 60 {
        conn.game_tick.current_control_frame += 1;
        conn.game_tick.last_update = time.last_update();
    }
}
