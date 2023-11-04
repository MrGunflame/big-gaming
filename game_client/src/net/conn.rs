use std::collections::VecDeque;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use ahash::HashMap;
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
use super::snapshot::MessageBacklog;
//use super::prediction::ClientPredictions;
use super::world::CommandBuffer;

#[derive(Debug)]
pub struct ServerConnection {
    pub world: WorldState,
    pub backlog: MessageBacklog,

    pub handle: Option<Arc<ConnectionHandle>>,
    pub host: EntityId,

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
    // Flag to indicate that the inventory actions need to be rebuilt.
    // FIXME: Replace with a more fine-grained update method.
    pub(crate) inventory_update: bool,
}

impl ServerConnection {
    pub fn new(config: &Config) -> Self {
        let mut world = WorldState::new();
        world.insert(ControlFrame(0));

        Self {
            handle: None,
            host: EntityId::dangling(),
            server_entities: Entities::new(),
            last_render_frame: None,
            trace: WorldTrace::new(),
            world,
            metrics: Metrics::default(),
            config: config.clone(),
            buffer: VecDeque::new(),
            input_buffer: InputBuffer::new(),
            physics: game_physics::Pipeline::new(),
            event_queue: EventQueue::new(),
            next_message_id: 0,
            modules: Modules::new(),
            inventory_update: false,
            backlog: MessageBacklog::new(8192),
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

        match inner(addr, ControlFrame(0), ControlFrame(0)) {
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

    pub fn send(&mut self, control_frame: ControlFrame, body: DataMessageBody) {
        if !self.is_connected() {
            tracing::warn!("attempted to send a command, but the peer is not connected");
            return;
        }

        let msg = DataMessage {
            id: MessageId(self.next_message_id),
            control_frame,
            body,
        };
        self.next_message_id += 1;

        self.input_buffer.push(msg.clone());
        self.buffer.push_back(msg);
    }

    pub fn shutdown(&mut self) {
        // The connection will automatically shut down after the last
        // handle was dropped.
        self.handle = None;
        self.buffer.clear();
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

    pub fn update(&mut self) {
        if !self.is_connected() {
            tracing::warn!("not connected");
            return;
        }

        self.flush_buffer();

        flush_command_queue(self);
    }
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
