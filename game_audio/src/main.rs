use std::time::Instant;

use bevy_app::App;
use game_audio::effects::Volume;
use game_audio::sound_data::{Settings, SoundData};
use game_audio::track::{Track, TrackId};
use game_audio::{AudioManager, AudioPlugin};

fn main() {
    pretty_env_logger::init();

    let mut manager = AudioManager::new();

    let mut data = SoundData::from_file("./../../x.ogg");

    let track = manager.add_track(Track {
        target: TrackId::Main,
        volume: Volume(0.2),
    });

    manager.play(data.clone(), Settings { track });

    let mut it = 0;

    let mut now = Instant::now();
    loop {
        manager.update();
        while now.elapsed().as_millis() <= 16 {
            std::thread::sleep_ms(1);
        }
        now = Instant::now();
        it += 1;

        if it == 1000 {
            dbg!("spawn");
            data.volume = Volume(5.0);
            manager.play(data.clone(), Settings { track });
        }
    }
}
