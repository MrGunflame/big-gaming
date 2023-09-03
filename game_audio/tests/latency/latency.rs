use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use game_audio::backend::Backend;
use game_audio::channel::Receiver;
use game_audio::effects::Volume;
use game_audio::sound::Frame;
use game_audio::sound_data::{Settings, SoundData};
use game_audio::AudioManager;

#[test]
fn test_latency() {
    let sound = SoundData {
        frames: vec![
            Frame {
                left: 1.0,
                right: 1.0,
            };
            48_000
        ],
        sample_rate: 48_000,
        volume: Volume::default(),
    };

    let backend = LatencyTestBackend::default();

    let mut manager = AudioManager::new(backend.clone());

    // Fill the initial buffer.
    for _ in 0..48_000 {
        manager.update();
    }

    manager.play(sound, Settings::default());

    let mut ticks = 0;
    let now = Instant::now();
    loop {
        manager.update();

        if backend.cell.load(Ordering::Acquire) {
            break;
        }
    }

    println!("Sound played after {}", ticks);
    println!("Sound played after {:?}", now.elapsed());
}

#[derive(Clone, Debug, Default)]
pub struct LatencyTestBackend {
    cell: Arc<AtomicBool>,
}

impl Backend for LatencyTestBackend {
    fn create_output_stream(&mut self, mut rx: Receiver) {
        let cell = self.cell.clone();

        std::thread::spawn(move || loop {
            if let Some(frame) = rx.recv() {
                if frame.left == 1.0 && frame.right == 1.0 {
                    cell.store(true, Ordering::Release);
                }
            }
        });
    }
}
