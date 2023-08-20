use core::num;
use std::time::Instant;

use bevy_ecs::system::{In, Resource};
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
        let mut index = 0;
        // 1.05 to keep a small buffer.
        let num_samples = (data.sample_rate as f64 * (1.0 / 60.0)) * 1.05;

        let mut now = Instant::now();
        loop {
            for _ in 0..num_samples as u32 {
                let Some(frame) = data.frames.get(index) else {
                    return;
                };

                self.tx.get_mut().push(*frame);
                now = Instant::now();
                index += 1;
            }

            //while now.elapsed().as_secs_f64() < 1.0 / data.sample_rate as f64 {}
            //std::thread::sleep_ms(16);
            while now.elapsed().as_millis() <= 16 {}
        }
    }

    pub fn update(&mut self) {}
}
