use bevy_ecs::system::{In, Resource};
use game_common::utils::exclusive::Exclusive;
use slotmap::SlotMap;

use crate::backend::DefaultBackend;
use crate::sound::{Frame, PlayingSound, Queue, Sender, SoundId};
use crate::sound_data::SoundData;

#[derive(Debug, Resource)]
pub struct AudioManager {
    backend: DefaultBackend,
    tx: Exclusive<Sender>,
    sounds: SlotMap<slotmap::DefaultKey, PlayingSound>,
    sample_rate: u32,
}

impl AudioManager {
    pub fn new() -> Self {
        let queue = Queue::new(100_000_000);
        let (tx, rx) = queue.split();

        let backend = DefaultBackend::new(rx);

        Self {
            backend,
            tx: Exclusive::new(tx),
            sounds: SlotMap::new(),
            sample_rate: 48_000,
        }
    }

    pub fn play(&mut self, data: SoundData) -> SoundId {
        let key = self.sounds.insert(PlayingSound { data, cursor: 0 });
        SoundId(key)
    }

    pub fn update(&mut self) {
        // 1.05 to keep a small buffer.
        let num_samples = (self.sample_rate as f64 * (1.0 / 60.0)) * 1.05;

        let mut buf = vec![Frame::EQUILIBRIUM; num_samples as usize];

        let mut drop_sounds = vec![];

        for (id, sound) in self.sounds.iter_mut() {
            for index in 0..num_samples as usize {
                let Some(frame) = sound.data.frames.get(sound.cursor) else {
                    drop_sounds.push(id);
                    break;
                };

                buf[index].left += frame.left * sound.data.volume.0;
                buf[index].right += frame.right * sound.data.volume.0;

                sound.cursor += 1;
            }
        }

        for elem in buf {
            if elem.left.abs() > 1.0 || elem.right.abs() > 1.0 {
                tracing::warn!("clipping");
            }

            self.tx.get_mut().push(elem);
        }

        for id in drop_sounds {
            self.sounds.remove(id);
        }
    }
}
