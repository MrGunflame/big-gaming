pub mod config;
pub mod conn;
pub mod entity;
pub mod net;
pub mod plugins;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod world;

use std::time::Duration;

use bevy::app::App;
use tokio::time::{interval, MissedTickBehavior};

use crate::server::Server;
use crate::state::State;

pub async fn run(mut app: App, state: State) {
    let timestep = Duration::from_secs(1) / state.config.timestep;

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

    let mut interval = interval(timestep.into());
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        app.update();
        interval.tick().await;
    }
}
