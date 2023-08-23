use std::sync::Arc;

use parking_lot::Mutex;

use crate::sound::Buffer;

use super::Backend;

pub struct StubBackend {}

impl StubBackend {
    pub fn new(buf: Arc<Mutex<Buffer>>) -> Self {
        Self {}
    }
}

impl Backend for StubBackend {
    fn create_output_stream(&mut self, buf: Arc<Mutex<Buffer>>) {}
}
