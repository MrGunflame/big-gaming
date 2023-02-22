use std::time::Duration;

use bevy::prelude::{App, Commands, Transform};
use clap::Parser;
use conn::Connections;
use game_common::archive::loader::ModuleLoader;
use game_common::archive::GameArchive;
use game_common::components::object::{Object, ObjectId};
use game_common::entity::{Entity, EntityData, EntityId};
use game_common::id::WeakId;
use game_core::CorePlugins;
use plugins::ServerPlugins;
use server::Server;
use snapshot::CommandQueue;
use state::State;
use tokio::time::{interval, MissedTickBehavior};

mod config;
mod conn;
mod entity;
mod plugins;
mod server;
mod snapshot;
mod state;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

#[tokio::main]
async fn main() {
    let state = State::new();

    let queue = state.queue.clone();
    let conns = state.conns.clone();

    tokio::task::spawn(async move {
        let server = Server::new(state).unwrap();
        server.await.unwrap();
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

    app.add_startup_system(setup);

    let mut interval = interval(Duration::from_millis(50));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        app.update();
        interval.tick().await;
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn(Transform::default())
        .insert(Object {
            id: ObjectId(0.into()),
        })
        .insert(Entity {
            id: EntityId::new(),
            transform: Transform::default(),
            data: EntityData::Object {
                id: ObjectId(0.into()),
            },
        });
}
