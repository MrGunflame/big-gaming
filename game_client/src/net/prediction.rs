use ahash::HashMap;
use game_common::world::control_frame::ControlFrame;
use game_net::message::{DataMessage, MessageId};

#[derive(Clone, Debug)]
pub struct InputBuffer {
    buffer: Vec<DataMessage>,
    // We need to buffer acknowledged messages until we render
    // that frame.
    removal_queue: HashMap<ControlFrame, Vec<MessageId>>,
    /// Last rendered control frame.
    last_cf: ControlFrame,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            removal_queue: HashMap::default(),
            last_cf: ControlFrame(0),
        }
    }

    pub fn push(&mut self, msg: DataMessage) {
        self.buffer.push(msg);
    }

    pub fn remove(&mut self, cf: ControlFrame, id: MessageId) {
        if cf <= self.last_cf {
            tracing::warn!("ACK for {:?} arrived too late, frame already rendered", cf);
            self.buffer.retain(|msg| msg.id != id);
        } else {
            self.removal_queue.entry(cf).or_default().push(id);
        }
    }

    pub fn clear(&mut self, render_cf: ControlFrame) {
        if let Some(ids) = self.removal_queue.remove(&render_cf) {
            for id in ids {
                self.buffer.retain(|msg| msg.id != id);
            }
        }

        self.last_cf = render_cf;
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataMessage> {
        self.buffer.iter()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
