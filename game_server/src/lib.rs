pub mod config;
pub mod conn;
pub mod net;
pub mod plugins;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod world;

use std::time::{Duration, Instant};

use ahash::HashMap;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_common::world::gen::Generator;
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::Modules;
use game_scene::scene2::{Key, SceneGraph};
use game_scene::SceneSpawner;
use game_script::Executor;
use game_tasks::TaskPool;
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
        let now = Instant::now();
        interval.wait(now).await;

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
    pub script_executor: Executor,
    pub pool: TaskPool,
    pub scene: SceneState,
}

impl ServerState {
    pub fn new(generator: Generator, modules: Modules, config: Config, executor: Executor) -> Self {
        Self {
            world: WorldState::new(),
            level: world::level::Level::new(generator),
            pipeline: game_physics::Pipeline::new(),
            event_queue: EventQueue::new(),
            modules,
            state: State::new(config),
            script_executor: executor,
            pool: TaskPool::new(8),
            scene: SceneState {
                spawner: SceneSpawner::default(),
                graph: SceneGraph::new(),
                entities: HashMap::default(),
            },
        }
    }
}

pub struct SceneState {
    spawner: SceneSpawner,
    graph: SceneGraph,
    entities: HashMap<Key, EntityId>,
}
