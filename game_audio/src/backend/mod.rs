mod cpal;
mod stub;

use std::sync::Arc;

pub use cpal::CpalBackend;
use parking_lot::Mutex;
pub use stub::StubBackend;

use crate::sound::Buffer;

pub type DefaultBackend = cpal::CpalBackend;

pub trait Backend {
    fn create_output_stream(&mut self, buf: Arc<Mutex<Buffer>>);
}
