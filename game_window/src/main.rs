use bevy_app::App;
use bevy_ecs::system::Commands;
use game_window::{Window, WindowPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugin(WindowPlugin);

    app.add_startup_system(setup);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Window {
        title: "Hello World!".to_owned(),
    });
}
