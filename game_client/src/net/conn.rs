use std::collections::VecDeque;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_common::world::control_frame::ControlFrame;
use game_common::world::world::{Snapshot, WorldState};
use game_core::counter::{Interval, IntervalImpl, UpdateCounter};
use game_core::modules::Modules;
use game_core::time::Time;
use game_net::conn::ConnectionHandle;
use game_net::message::{DataMessage, DataMessageBody, MessageId};
use game_script::effect::Effect;
use game_script::executor::ScriptExecutor;
use game_script::Context;
use game_tracing::world::WorldTrace;

use crate::config::Config;
use crate::net::socket::spawn_conn;

use super::entities::Entities;
use super::flush_command_queue;
use super::prediction::InputBuffer;
//use super::prediction::ClientPredictions;
use super::world::{apply_world_delta, Command, CommandBuffer};

#[derive(Debug)]
pub struct ServerConnection<I> {
    pub world: WorldState,
    pub current_state: Option<Snapshot>,

    pub handle: Option<Arc<ConnectionHandle>>,
    pub host: EntityId,

    pub game_tick: GameTick<I>,

    /// How many frames to backlog and interpolate over.
    interplation_frames: ControlFrame,

    pub server_entities: Entities,

    /// The previously rendered frame, `None` if not rendered yet.
    pub last_render_frame: Option<ControlFrame>,

    pub trace: WorldTrace,

    pub metrics: Metrics,
    pub config: Config,

    buffer: VecDeque<DataMessage>,

    pub(crate) input_buffer: InputBuffer,

    pub(crate) physics: game_physics::Pipeline,
    pub(crate) event_queue: EventQueue,
    next_message_id: u32,
    pub(crate) modules: Modules,
}

impl<I> ServerConnection<I> {
    pub fn new_with_interval(config: &Config, interval: I) -> Self {
        let mut world = WorldState::new();
        world.insert(ControlFrame(0));

        Self {
            handle: None,
            host: EntityId::dangling(),
            game_tick: GameTick {
                interval,
                current_control_frame: ControlFrame(0),
                initial_idle_passed: false,
                counter: UpdateCounter::new(),
            },
            interplation_frames: ControlFrame(config.network.interpolation_frames),
            server_entities: Entities::new(),
            last_render_frame: None,
            trace: WorldTrace::new(),
            world,
            metrics: Metrics::default(),
            config: config.clone(),
            buffer: VecDeque::new(),
            input_buffer: InputBuffer::new(),
            current_state: None,
            physics: game_physics::Pipeline::new(),
            event_queue: EventQueue::new(),
            next_message_id: 0,
            modules: Modules::new(),
        }
    }

    pub fn connect<T>(&mut self, addr: T)
    where
        T: ToSocketAddrs,
    {
        fn inner(
            addr: impl ToSocketAddrs,
            cf: ControlFrame,
            const_delay: ControlFrame,
        ) -> Result<Arc<ConnectionHandle>, Box<dyn std::error::Error + Send + Sync + 'static>>
        {
            // TODO: Use async API
            let addr = match addr.to_socket_addrs()?.nth(0) {
                Some(addr) => addr,
                None => panic!("empty dns result"),
            };

            spawn_conn(addr, cf, const_delay)
        }

        match inner(
            addr,
            // Note that we always start on the "next" frame.
            // The first frame must be empty to bootstrap the
            // first interpolation tick.
            self.game_tick.current_control_frame + 1,
            self.interplation_frames,
        ) {
            Ok(handle) => {
                self.handle = Some(handle);
            }
            Err(err) => {
                tracing::error!("failed to connect: {}", err);
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.handle.is_some()
    }

    pub fn send(&mut self, body: DataMessageBody) {
        if !self.is_connected() {
            tracing::warn!("attempted to send a command, but the peer is not connected");
            return;
        }

        let msg = DataMessage {
            id: MessageId(self.next_message_id),
            control_frame: self.game_tick.current_control_frame,
            body,
        };
        self.next_message_id += 1;

        self.input_buffer.push(msg.clone());
        self.buffer.push_back(msg);
    }

    pub fn shutdown(&mut self) {
        dbg!("shutdown");
        // The connection will automatically shut down after the last
        // handle was dropped.
        self.handle = None;
        self.buffer.clear();
    }

    /// Returns the current control frame.
    pub fn control_frame(&mut self) -> CurrentControlFrame {
        let interpolation_period = self.interplation_frames;

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

    fn flush_buffer(&mut self) {
        let Some(handle) = &self.handle else {
            tracing::error!("not connected");
            return;
        };

        for msg in self.buffer.drain(..) {
            handle.send(msg);
        }
    }

    fn step_physics(&mut self, cmd_buffer: &mut CommandBuffer) {
        let mut world = WorldState::from_snapshot(self.current_state.clone().unwrap());
        let mut view = world.front_mut().unwrap();
        self.physics.step(&mut view, &mut self.event_queue);
    }

    fn run_scripts(&mut self, executor: &ScriptExecutor, cmd_buffer: &mut CommandBuffer) {
        let mut world = WorldState::from_snapshot(self.current_state.clone().unwrap());
        let mut view = world.front_mut().unwrap();

        let effects = executor.run(Context {
            view: &mut view,
            physics_pipeline: &self.physics,
            events: &mut self.event_queue,
        });

        for effect in effects.into_iter() {
            match effect {
                Effect::EntitySpawn(entity) => {
                    todo!()
                }
                Effect::EntityDespawn(id) => todo!(),
                Effect::EntityTranslate(id, translation) => {
                    cmd_buffer.push(Command::Translate {
                        entity: id,
                        start: ControlFrame(0),
                        end: ControlFrame(0),
                        dst: translation,
                    });

                    self.current_state
                        .as_mut()
                        .unwrap()
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .transform
                        .translation = translation;
                }
                Effect::EntityRotate(id, rotation) => {
                    cmd_buffer.push(Command::Rotate {
                        entity: id,
                        start: ControlFrame(0),
                        end: ControlFrame(0),
                        dst: rotation,
                    });

                    self.current_state
                        .as_mut()
                        .unwrap()
                        .entities
                        .get_mut(id)
                        .unwrap()
                        .transform
                        .rotation = rotation;
                }
                _ => todo!(),
            }
        }
    }
}

impl<I> ServerConnection<I>
where
    I: IntervalImpl,
{
    pub fn update(&mut self, time: &Time, buffer: &mut CommandBuffer, executor: &ScriptExecutor) {
        if !self.is_connected() {
            return;
        }

        while self.game_tick.interval.is_ready(time.last_update()) {
            // Flush input buffer from previous frame.
            self.flush_buffer();

            self.game_tick.current_control_frame += 1;
            self.game_tick.counter.update();

            // debug_assert!(self
            //     .world
            //     .get(self.game_tick.current_control_frame)
            //     .is_none());
            if self
                .world
                .get(self.game_tick.current_control_frame)
                .is_none()
            {
                self.world.insert(self.game_tick.current_control_frame);
            }

            // Snapshots render..head should now exist.
            if cfg!(debug_assertions) {
                let control_frame = self.control_frame();
                let mut start = match control_frame.render {
                    Some(render) => render,
                    None => ControlFrame(0),
                };
                let end = control_frame.head;

                while start != end + 1 {
                    assert!(self.world.get(start).is_some());

                    start += 1;
                }
            }

            tracing::debug!(
                "Stepping control frame to {:?} (UPS = {})",
                self.game_tick.current_control_frame,
                self.game_tick.counter.ups(),
            );

            flush_command_queue(self);
            apply_world_delta(self, buffer, executor);

            if self.current_state.is_some() {
                self.step_physics(buffer);
                self.run_scripts(executor, buffer);
            }
        }
    }
}

impl ServerConnection<Interval> {
    pub fn new(config: &Config) -> Self {
        let interval = Interval::new(Duration::from_secs(1) / config.timestep);
        Self::new_with_interval(config, interval)
    }
}

#[derive(Debug)]
pub struct GameTick<I> {
    pub interval: I,
    current_control_frame: ControlFrame,
    /// Whether the initial idle phase passed. In this phase the renderer is waiting for the
    /// initial interpolation window to build up.
    // TODO: Maybe make this AtomicBool to prevent `control_frame()` being `&mut self`.
    initial_idle_passed: bool,
    counter: UpdateCounter,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CurrentControlFrame {
    /// The newest snapshot of the world.
    pub head: ControlFrame,
    /// The snapshot of the world that should be rendered, `None` if not ready.
    pub render: Option<ControlFrame>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Metrics {
    /// Commands sent to the server.
    pub commands_sent: u64,
    /// Commands acknowledged by the server.
    pub commands_acks: u64,
}
