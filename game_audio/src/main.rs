use std::time::Instant;

use bevy_app::App;
use game_audio::sound_data::SoundData;
use game_audio::{AudioManager, AudioPlugin};

fn main() {
    pretty_env_logger::init();

    let mut manager = AudioManager::new();

    let data = SoundData::from_file("./../../x.ogg");
    manager.play(data.clone());

    //manager.update();
    manager.play(data);

    let mut now = Instant::now();
    loop {
        manager.update();
        while now.elapsed().as_millis() <= 16 {}
        now = Instant::now();
    }
}
