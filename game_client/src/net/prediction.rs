use game_common::world::control_frame::ControlFrame;
use game_net::message::DataMessage;

#[derive(Clone, Debug)]
pub struct InputBuffer {
    buffer: Vec<DataMessage>,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push(&mut self, msg: DataMessage) {
        self.buffer.push(msg);
    }

    /// Remove all inputs up to [`ControlFrame`].
    pub fn remove(&mut self, control_frame: ControlFrame) {
        self.buffer.retain(|msg| msg.control_frame > control_frame);
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataMessage> {
        self.buffer.iter()
    }
}
