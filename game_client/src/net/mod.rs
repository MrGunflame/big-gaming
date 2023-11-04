mod conn;
mod entities;
mod prediction;
mod snapshot;
mod socket;
pub mod world;

use std::collections::VecDeque;

use game_net::message::{ControlMessage, Message};

pub use self::conn::ServerConnection;
pub use self::entities::Entities;

fn flush_command_queue(conn: &mut ServerConnection) {
    let mut queue = VecDeque::new();
    let handle = conn.handle.as_ref().unwrap();
    while let Some(msg) = handle.recv() {
        queue.push_back(msg);
    }

    while let Some(msg) = queue.pop_front() {
        let msg = match msg {
            Message::Control(ControlMessage::Connected()) => {
                continue;
            }
            Message::Control(ControlMessage::Disconnected) => {
                conn.shutdown();
                continue;
            }
            Message::Control(ControlMessage::Acknowledge(id, cf)) => {
                conn.input_buffer.remove(cf, id);
                continue;
            }
            Message::Data(msg) => msg,
        };

        conn.backlog.insert(msg.control_frame, msg);
    }
}
