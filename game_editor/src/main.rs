use bevy::prelude::App;
use bevy::DefaultPlugins;

mod plugins;

fn main() {
    App::new().add_plugins(DefaultPlugins).run();
}
