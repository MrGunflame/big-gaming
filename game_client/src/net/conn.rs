use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{ResMut, Resource};
use bevy_ecs::world::{FromWorld, World};
use game_common::entity::{EntityId, EntityMap};
use game_common::world::control_frame::ControlFrame;
use game_common::world::world::WorldState;
use game_core::counter::UpdateCounter;
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
                interval: Interval::new(),
                current_control_frame: ControlFrame(0),
                initial_idle_passed: false,
                counter: UpdateCounter::new(),
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
            cf: ControlFrame,
        ) -> Result<ConnectionHandle, Box<dyn std::error::Error + Send + Sync + 'static>> {
            // TODO: Use async API
            let addr = match addr.to_socket_addrs()?.nth(0) {
                Some(addr) => addr,
                None => panic!("empty dns result"),
            };

            super::spawn_conn(queue, addr, cf)
        }

        match inner(
            self.queue.clone(),
            addr,
            self.game_tick.current_control_frame,
        ) {
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
    pub fn control_frame(&mut self) -> CurrentControlFrame {
        // Render interpolation period of 100ms.
        let interpolation_period = ControlFrame(6);

        let head = self.game_tick.current_control_frame;

        // If the initial idle phase passed, ControlFrame wraps around.
        let render = if self.game_tick.initial_idle_passed {
            Some(head - interpolation_period)
        } else {
            if let Some(cf) = head.checked_sub(interpolation_period) {
                self.game_tick.initial_idle_passed = true;
                Some(cf)
            } else {
                None
            }
        };

        CurrentControlFrame { head, render }
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
    interval: Interval,
    current_control_frame: ControlFrame,
    /// Whether the initial idle phase passed. In this phase the renderer is waiting for the
    /// initial interpolation window to build up.
    // TODO: Maybe make this AtomicBool to prevent `control_frame()` being `&mut self`.
    initial_idle_passed: bool,
    counter: UpdateCounter,
}

pub fn tick_game(
    time: ResMut<Time>,
    mut conn: ResMut<ServerConnection>,
    mut world: ResMut<WorldState>,
) {
    while conn.game_tick.interval.is_ready(time.last_update()) {
        conn.game_tick.current_control_frame += 1;
        conn.game_tick.counter.update();

        debug_assert!(world.get(conn.game_tick.current_control_frame).is_none());
        world.insert(conn.game_tick.current_control_frame);

        // Snapshots render..head should now exist.
        if cfg!(debug_assertions) {
            let control_frame = conn.control_frame();
            let mut start = match control_frame.render {
                Some(render) => render,
                None => ControlFrame(0),
            };
            let end = control_frame.head;

            while start != end + 1 {
                assert!(world.get(start).is_some());

                start += 1;
            }
        }

        tracing::info!(
            "Stepping control frame to {:?} (UPS = {})",
            conn.game_tick.current_control_frame,
            conn.game_tick.counter.ups(),
        );
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CurrentControlFrame {
    /// The newest snapshot of the world.
    pub head: ControlFrame,
    /// The snapshot of the world that should be rendered, `None` if not ready.
    pub render: Option<ControlFrame>,
}

#[derive(Debug)]
struct Interval {
    last_update: Instant,
    /// The uniform timestep duration of a control frame.
    timestep: Duration,
}

impl Interval {
    fn new() -> Self {
        Self {
            last_update: Instant::now(),
            timestep: Duration::from_secs(1) / 60,
        }
    }

    fn is_ready(&mut self, now: Instant) -> bool {
        let elapsed = now - self.last_update;

        if elapsed >= self.timestep {
            self.last_update += self.timestep;
            true
        } else {
            false
        }
    }
}
