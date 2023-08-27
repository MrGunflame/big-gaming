use std::time::Instant;

use bevy_app::App;
use game_audio::backend::DefaultBackend;
use game_audio::effects::Volume;
use game_audio::sound_data::{Settings, SoundData};
use game_audio::track::{Track, TrackId};
use game_audio::{AudioManager, AudioPlugin};

fn main() {
    let mut manager = AudioManager::new(DefaultBackend::new());

    let mut data = SoundData::from_file("./../../x.ogg");

    let track = manager.add_track(Track {
        target: TrackId::Main,
        volume: Volume(0.2),
    });

    manager.play(
        data.clone(),
        Settings {
            destination: track.into(),
        },
    );

    let now = Instant::now();
    let mut spawned = false;
    loop {
        manager.update();

        if now.elapsed().as_millis() > 10000 && !spawned {
            dbg!("spawn");
            data.volume = Volume(5.0);
            manager.play(
                data.clone(),
                Settings {
                    destination: track.into(),
                },
            );
            spawned = true;
        }
    }
}
