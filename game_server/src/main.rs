use std::time::Duration;

use bevy::prelude::{App, Commands};
use bevy::transform::TransformBundle;
use bevy_rapier3d::prelude::{Collider, RigidBody};
use clap::Parser;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_core::CorePlugins;
use plugins::ServerPlugins;
use server::Server;
use state::State;
use tokio::time::{interval, MissedTickBehavior};
use world::WorldPlugin;

mod config;
mod conn;
mod entity;
mod net;
pub mod plugins;
mod server;
mod snapshot;
mod state;
mod world;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

#[tokio::main]
async fn main() {
    game_core::logger::init();

    let state = State::new();

    let queue = state.queue.clone();
    let conns = state.conns.clone();

    tokio::task::spawn(async move {
        let server = match Server::new(state) {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("failed to run server: {}", err);
                return;
            }
        };

        if let Err(err) = server.await {
            tracing::error!("failed to run server: {}", err);
        }
    });

    let archive = GameArchive::new();

    let loader = ModuleLoader::new(&archive);
    loader.load("../mods/core").unwrap();

    let mut app = App::new();
    app.insert_resource(archive);
    app.add_plugin(CorePlugins);

    app.insert_resource(queue);
    app.insert_resource(conns);
    app.add_plugin(ServerPlugins);

    app.add_plugin(WorldPlugin);

    app.add_startup_system(setup);

    let timestep = Duration::from_secs(1) / 60;

    let mut interval = interval(timestep.into());
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        app.update();
        interval.tick().await;
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn(TransformBundle::default())
        .insert(RigidBody::Fixed)
        .insert(Collider::cuboid(1000.0, 0.1, 1000.0));

    // commands
    //     .spawn(Transform::default())
    //     .insert(Object {
    //         id: ObjectId(0.into()),
    //     })
    //     .insert(Entity {
    //         id: EntityId::new(),
    //         transform: Transform::default(),
    //         data: EntityData::Object {
    //             id: ObjectId(0.into()),
    //         },
    //     });
}
