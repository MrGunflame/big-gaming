pub mod config;
pub mod conn;
pub mod net;
pub mod plugins;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod world;

use std::time::{Duration, Instant};

use game_common::events::EventQueue;
use game_common::world::gen::Generator;
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::Modules;
use game_script::executor::ScriptExecutor;
use game_script::scripts::RecordTargets;
use game_script::ScriptServer;
use tracing::{span, Level};
use world::state::WorldState;

use crate::config::Config;
use crate::plugins::tick;
use crate::server::Server;
use crate::state::State;

pub async fn run(mut state: ServerState) {
    let root = span!(Level::INFO, "Server");
    let _guard = root.enter();

    let timestep = Duration::from_secs(1) / state.state.config.timestep;

    {
        let state = state.state.clone();
        tokio::task::spawn(async move {
            let server = match Server::new(state.clone()) {
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
    }

    let mut interval = Interval::new(timestep);

    let mut ups = UpdateCounter::new();
    loop {
        while !interval.is_ready(Instant::now()) {
            continue;
        }

        tick(&mut state);

        let mut cf = state.state.control_frame.lock();
        *cf += 1;

        ups.update();

        tracing::debug!("Stepping Control frame to {:?} (UPS = {})", cf, ups.ups());
    }
}

pub struct ServerState {
    pub world: WorldState,
    pub level: world::level::Level,
    pub pipeline: game_physics::Pipeline,
    pub event_queue: EventQueue,
    pub modules: Modules,
    pub state: State,
    pub script_executor: ScriptExecutor,
}

impl ServerState {
    pub fn new(
        generator: Generator,
        modules: Modules,
        config: Config,
        server: ScriptServer,
        record_targets: RecordTargets,
    ) -> Self {
        Self {
            world: WorldState::new(),
            level: world::level::Level::new(generator),
            pipeline: game_physics::Pipeline::new(),
            event_queue: EventQueue::new(),
            modules,
            state: State::new(config),
            script_executor: ScriptExecutor::new(server, record_targets),
        }
    }
}
