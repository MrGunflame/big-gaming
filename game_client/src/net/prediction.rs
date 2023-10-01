use ahash::HashMap;
use game_common::world::control_frame::ControlFrame;
use game_net::message::{DataMessage, MessageId};

#[derive(Clone, Debug)]
pub struct InputBuffer {
    buffer: Vec<DataMessage>,
    // We need to buffer acknowledged messages until we render
    // that frame.
    removal_queue: HashMap<ControlFrame, Vec<MessageId>>,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            removal_queue: HashMap::default(),
        }
    }

    pub fn push(&mut self, msg: DataMessage) {
        self.buffer.push(msg);
    }

    pub fn remove(&mut self, cf: ControlFrame, id: MessageId) {
        self.removal_queue.entry(cf).or_default().push(id);
    }

    pub fn clear(&mut self, render_cf: ControlFrame) {
        if let Some(ids) = self.removal_queue.remove(&render_cf) {
            for id in ids {
                self.buffer.retain(|msg| msg.id != id);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataMessage> {
        self.buffer.iter()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}
