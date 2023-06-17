#![feature(const_trait_impl)]
#![feature(const_option)]
#![deny(unsafe_op_in_unsafe_fn)]

mod assets;
mod net;
mod plugins;
mod prev_transform;
mod state;
mod utils;
mod window;
mod world;

use bevy_app::App;
use clap::Parser;
use game_core::CorePlugins;
use game_render::RenderPlugin;
use net::NetPlugin;
use plugins::actions::ActionsPlugin;
use state::InternalGameState;

#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    connect: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut app = App::new();

    app.add_plugin(RenderPlugin);

    app.init_resource::<InternalGameState>();

    // Window setup
    app.add_startup_system(window::spawn_primary_window);
    app.add_system(window::destroy_primary_window);

    app.add_plugin(CorePlugins);
    app.add_plugin(NetPlugin::default());
    app.add_plugin(ActionsPlugin);

    game_core::modules::load_modules(&mut app);

    if let Some(addr) = args.connect {
        tracing::info!("Connecting to {}", addr);
    }

    app.run();
}
