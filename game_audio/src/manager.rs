use std::time::Instant;

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
        let queue = Queue::new(100_000_000);
        let (tx, rx) = queue.split();

        let backend = DefaultBackend::new(rx);

        Self {
            backend,
            tx: Exclusive::new(tx),
        }
    }

    pub fn play(&mut self, data: SoundData) {
        let mut now = Instant::now();
        let mut index = 0;
        dbg!(&data.sample_rate);
        loop {
            //while now.elapsed().as_secs_f64() < 1.0 / data.sample_rate as f64 {}
            let Some(frame) = data.frames.get(index) else {
                break;
            };

            self.tx.get_mut().push(*frame);
            now = Instant::now();
            index += 1;
        }
    }

    pub fn update(&mut self) {}
}
