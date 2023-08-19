use bevy_app::App;
use game_audio::AudioPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugin(AudioPlugin);
}
