use std::time::Instant;

use bevy_app::App;
use game_audio::sound_data::SoundData;
use game_audio::{AudioManager, AudioPlugin};

fn main() {
    pretty_env_logger::init();

    dbg!("0");
    let mut manager = AudioManager::new();
    dbg!("1");

    let data = SoundData::from_file("./../../x.ogg");
    dbg!("2");
    manager.play(data.clone(), Default::default());
    dbg!("3");

    //manager.update();
    //manager.play(data, Default::default());

    let mut now = Instant::now();
    loop {
        manager.update();
        while now.elapsed().as_millis() <= 16 {}
        now = Instant::now();
    }
}
