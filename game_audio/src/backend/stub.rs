use crate::channel::Receiver;

use super::Backend;

#[derive(Copy, Clone, Debug, Default)]
pub struct StubBackend;

impl Backend for StubBackend {
    fn create_output_stream(&mut self, rx: Receiver) {}
}
