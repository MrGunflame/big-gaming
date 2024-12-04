mod cpal;
pub mod pipewire;
mod stub;

pub use cpal::CpalBackend;
pub use stub::StubBackend;

use crate::channel::Receiver;

pub trait Backend {
    fn create_output_stream(&mut self, rx: Receiver);
}

#[derive(Debug)]
pub enum DefaultBackend {
    Cpal(CpalBackend),
    Stub(StubBackend),
}

impl DefaultBackend {
    pub fn new() -> Self {
        match CpalBackend::new() {
            Ok(backend) => return Self::Cpal(backend),
            Err(err) => tracing::warn!("failed to create cpal backend: {}", err),
        }

        tracing::error!("failed to create audio backend");
        Self::Stub(StubBackend)
    }
}

impl Default for DefaultBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for DefaultBackend {
    fn create_output_stream(&mut self, rx: Receiver) {
        match self {
            Self::Cpal(backend) => backend.create_output_stream(rx),
            Self::Stub(backend) => backend.create_output_stream(rx),
        }
    }
}
