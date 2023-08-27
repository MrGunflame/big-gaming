use std::f32::consts::PI;
use std::sync::Arc;

use game_audio::backend::Backend;
use game_audio::effects::Volume;
use game_audio::sound::Frame;
use game_audio::sound_data::{Settings, SoundData};
use game_audio::AudioManager;
use parking_lot::Mutex;

#[test]
fn basic_playback() {
    let sound = create_test_sound();

    let backend = TestBackend {
        output: sound.frames.clone(),
        exit: Arc::new(Mutex::new(Output::Running)),
    };
    let mut manager = AudioManager::new(backend.clone());

    manager.play(sound, Settings::default());

    loop {
        let output = backend.exit.lock().clone();

        match output {
            Output::Running => manager.update(),
            Output::Ok => return,
            Output::Failed(msg) => panic!("{}", msg),
        }
    }
}

fn create_test_sound() -> SoundData {
    let sample_rate = 48_000;
    let mut sample_clock = 0.0;

    let frames = (0..sample_rate)
        .map(|_| {
            sample_clock = (sample_clock + 1.0) % sample_rate as f32;
            let val = (sample_clock * 440.0 * 2.0 * PI / sample_rate as f32).sin();
            Frame {
                left: val,
                right: val,
            }
        })
        .collect();

    SoundData {
        frames,
        sample_rate,
        volume: Volume::default(),
    }
}

#[derive(Clone, Debug)]
pub struct TestBackend {
    output: Vec<Frame>,
    exit: Arc<Mutex<Output>>,
}

impl Backend for TestBackend {
    fn create_output_stream(&mut self, mut rx: game_audio::queue::Receiver) {
        let output = self.output.clone();
        let mut index = 0;
        let exit = self.exit.clone();

        std::thread::spawn(move || loop {
            while let Some(frame) = rx.recv() {
                let Some(expected) = output.get(index).copied() else {
                    *exit.lock() = Output::Ok;
                    return;
                };

                if frame != expected {
                    *exit.lock() = Output::Failed(format!(
                        "missmatched frames at index {}: expected {:?}, got {:?}",
                        index, expected, frame
                    ));
                }

                index += 1;
            }
        });
    }
}

#[derive(Clone, Debug)]
enum Output {
    Running,
    Ok,
    Failed(String),
}
