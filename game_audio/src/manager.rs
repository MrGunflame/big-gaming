use bevy_ecs::system::Resource;
use game_common::utils::exclusive::Exclusive;

use crate::backend::DefaultBackend;
use crate::sound::{Queue, Sender};
use crate::sound_data::SoundData;

#[derive(Debug, Resource)]
pub struct AudioManager {
    backend: DefaultBackend,
    tx: Exclusive<Sender>,
}

impl AudioManager {
    pub fn new() -> Self {
        let queue = Queue::new(48_000);
        let (tx, rx) = queue.split();

        let backend = DefaultBackend::new(rx);

        Self {
            backend,
            tx: Exclusive::new(tx),
        }
    }

    pub fn play(&mut self, data: SoundData) {}
}
