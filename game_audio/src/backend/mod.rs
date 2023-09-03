mod cpal;
mod stub;

pub use cpal::CpalBackend;
pub use stub::StubBackend;

use crate::channel::Receiver;

pub type DefaultBackend = cpal::CpalBackend;

pub trait Backend {
    fn create_output_stream(&mut self, rx: Receiver);
}
