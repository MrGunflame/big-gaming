use bevy_app::App;
use bevy_ecs::system::Commands;
use game_render::RenderPlugin;
use game_window::{Window, WindowPlugin};
use tokio::runtime::{Builder, Runtime};

fn main() {
    pretty_env_logger::init();

    let mut app = App::new();
    app.add_plugin(WindowPlugin);
    app.add_plugin(RenderPlugin);

    app.add_startup_system(setup);

    // let rt = Builder::new_current_thread().build().unwrap();

    // rt.block_on(run());

    app.run();
}

fn setup(mut cmds: Commands) {
    cmds.spawn(Window {
        title: "Hello World!".to_owned(),
    });
}
