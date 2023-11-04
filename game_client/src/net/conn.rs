use std::collections::VecDeque;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use game_common::world::control_frame::ControlFrame;
use game_net::conn::ConnectionHandle;
use game_net::message::{DataMessage, DataMessageBody, MessageId};

use crate::net::socket::spawn_conn;

use super::flush_command_queue;
use super::prediction::InputBuffer;
use super::snapshot::MessageBacklog;
//use super::prediction::ClientPredictions;

#[derive(Debug)]
pub struct ServerConnection {
    pub backlog: MessageBacklog,
    pub handle: Option<Arc<ConnectionHandle>>,
    pub(crate) input_buffer: InputBuffer,
    buffer: VecDeque<DataMessage>,
    next_message_id: u32,
}

impl ServerConnection {
    pub fn new() -> Self {
        Self {
            handle: None,
            buffer: VecDeque::new(),
            input_buffer: InputBuffer::new(),
            next_message_id: 0,
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
