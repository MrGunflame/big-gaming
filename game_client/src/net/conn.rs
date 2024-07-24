use std::collections::VecDeque;
use std::time::Duration;

use game_common::world::control_frame::ControlFrame;
use game_net::conn::ConnectionHandle;
use game_net::message::{ControlMessage, DataMessage, DataMessageBody, Message, MessageId};
use tokio::sync::mpsc::error::TrySendError;

use super::prediction::InputBuffer;
use super::snapshot::MessageBacklog;

#[derive(Debug)]
pub struct ServerConnection {
    pub backlog: MessageBacklog,
    handle: ConnectionHandle,
    pub(crate) input_buffer: InputBuffer,
    buffer: VecDeque<DataMessage>,
    next_message_id: u32,
    pub(crate) latest_cf: ControlFrame,
}

impl ServerConnection {
    pub fn new(handle: ConnectionHandle) -> Self {
        Self {
            handle: handle,
            buffer: VecDeque::new(),
            input_buffer: InputBuffer::new(),
            backlog: MessageBacklog::new(8192),
            next_message_id: 0,
            latest_cf: ControlFrame(0),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.handle.is_connected()
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
        self.next_message_id = self.next_message_id.wrapping_add(1);

        self.input_buffer.push(msg.clone());
        self.buffer.push_back(msg);
    }

    pub fn rtt(&self) -> Duration {
        self.handle.rtt()
    }

    pub fn set_cf(&self, cf: ControlFrame) {
        self.handle.set_cf(cf);
    }

    pub fn shutdown(&mut self) {
        // The connection will automatically shut down after the last
        // handle was dropped.
        self.buffer.clear();
    }

    pub fn update(&mut self) {
        self.flush_outgoing_buffer();
        self.queue_incoming_messages();
    }

    fn flush_outgoing_buffer(&mut self) {
        while let Some(msg) = self.buffer.pop_front() {
            match self.handle.send(msg) {
                Ok(()) => (),
                Err(TrySendError::Full(msg)) => {
                    self.buffer.push_front(msg);
                    tracing::warn!("TX buffer is full, buffering until next tick");
                    break;
                }
                // Receiver dropped, i.e. we are no longer connected.
                Err(TrySendError::Closed(_)) => {
                    self.shutdown();
                    break;
                }
            }
        }
    }

    fn queue_incoming_messages(&mut self) {
        while let Some(msg) = self.handle.recv() {
            let msg = match msg {
                Message::Control(ControlMessage::Connected()) => {
                    continue;
                }
                Message::Control(ControlMessage::Disconnected) => {
                    self.shutdown();
                    return;
                }
                Message::Control(ControlMessage::Ack(cf)) => {
                    self.latest_cf = cf;
                    continue;
                }
                Message::Control(ControlMessage::Acknowledge(id, cf)) => {
                    self.input_buffer.remove(cf, id);
                    continue;
                }
                Message::Data(msg) => msg,
            };

            self.backlog.insert(msg.control_frame, msg);
        }
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
