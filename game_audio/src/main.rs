use bevy_app::App;
use game_audio::sound_data::SoundData;
use game_audio::{AudioManager, AudioPlugin};

fn main() {
    let mut manager = AudioManager::new();

    let data = SoundData::from_file("./../../x.ogg");
    manager.play(data);

    loop {}
}
