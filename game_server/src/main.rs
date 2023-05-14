use bevy_app::App;
use clap::Parser;

use game_core::modules;
use game_server::config::Config;
use tokio::runtime::Runtime;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

fn main() {
    game_core::logger::init();

    let config = Config::from_file("./config.toml").unwrap();

    let mut app = App::new();

    game_server::prepare(&mut app);

    modules::load_modules(&mut app);

    let rt = Runtime::new().unwrap();
    rt.block_on(game_server::run(app, config));
}
