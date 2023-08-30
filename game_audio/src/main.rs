use std::time::Instant;

use bevy_app::App;
use bevy_ecs::system::adapter::dbg;
use game_audio::backend::DefaultBackend;
use game_audio::effects::Volume;
use game_audio::sound_data::{Settings, SoundData};
use game_audio::spatial::{Emitter, Listener};
use game_audio::track::{Track, TrackId};
use game_audio::{AudioManager, AudioPlugin};
use glam::{Quat, Vec3};

fn main() {
    let mut manager = AudioManager::new(DefaultBackend::new());

    let mut data = SoundData::from_file("./../../x.ogg");

    let track = manager.add_track(Track {
        target: TrackId::Main,
        volume: Volume(1.0),
    });

    let listener = Listener {
        track,
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
    };

    let emitter = Emitter {
        translation: Vec3::new(0.0, 0.0, 2.0),
    };

    let listener_id = manager.add_listener(listener);
    let emitter_id = manager.add_emitter(emitter);

    manager.play(
        data.clone(),
        Settings {
            destination: emitter_id.into(),
        },
    );

    let mut rotation = Quat::IDENTITY;
    let distance = 2.0;

    let mut now = Instant::now();
    let mut spawned = false;
    loop {
        manager.update();

        let delta = now.elapsed().as_secs_f32();
        rotation = Quat::from_axis_angle(Vec3::Y, 10.0 * delta) * rotation;

        let emitter = manager.get_emitter_mut(emitter_id).unwrap();
        emitter.translation = listener.translation + rotation * Vec3::new(0.0, 0.0, distance);

        dbg!(emitter.translation);

        // if now.elapsed().as_millis() > 100_000 && !spawned {
        //  dbg!("spawn");
        //  data.volume = Volume(1.0);
        //  manager.play(
        //     data.clone(),
        //    Settings {
        //         destination: track.into(),
        //      },
        //   );
        //    spawned = true;
        // }

        now = Instant::now();
    }
}
