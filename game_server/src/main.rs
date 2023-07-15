use bevy_app::App;
use clap::Parser;

use game_core::modules;
use game_server::config::Config;
use tokio::runtime::{Builder, UnhandledPanic};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

fn main() {
    game_core::logger::init();

    let mut config_path = std::env::current_dir().unwrap();
    config_path.push("config.toml");

    let config = match Config::from_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            fatal!("failed to load config file from {:?}: {}", config_path, err);
        }
    };

    let mut app = App::new();

    game_server::prepare(&mut app);

    modules::load_modules(&mut app);

    let rt = Builder::new_multi_thread()
        .enable_all()
        .unhandled_panic(UnhandledPanic::ShutdownRuntime)
        .build()
        .unwrap();
    rt.block_on(game_server::run(app, config));
}

#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {{
        tracing::error!($($arg)*);
        tracing::error!("encountered fatal error, exiting");
        std::process::exit(1);
    }};
}
