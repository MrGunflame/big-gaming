pub mod command;
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
use command::Command;
use game_common::entity::EntityId;
use game_common::events::EventQueue;
use game_common::world::gen::Generator;
use game_core::command::ServerCommand;
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::Modules;
use game_scene::scene2::{Key, SceneGraph};
use game_scene::SceneSpawner;
use game_script::Executor;
use game_tasks::TaskPool;
use tokio::sync::{mpsc, oneshot};
use tracing::{span, trace_span, Level};
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

        process_commands(&mut state);
        tick(&mut state);

        let mut cf = state.state.control_frame.lock();
        *cf += 1;

        ups.update();

        tracing::debug!("Stepping Control frame to {:?} (UPS = {})", cf, ups.ups());
    }
}

pub struct ServerState {
    /// Start time of the server.
    pub start: Instant,
    pub command_queue: mpsc::Receiver<(Command, oneshot::Sender<String>)>,
    pub world: WorldState,
    pub level: world::level::Level,
    pub pipeline: game_physics::Pipeline,
    pub event_queue: EventQueue,
    pub modules: Modules,
    pub state: State,
    pub script_executor: Executor,
    pub pool: TaskPool,
    pub scene: SceneState,
    pub next_player: u64,
}

impl ServerState {
    pub fn new(
        command_handler: mpsc::Receiver<(Command, oneshot::Sender<String>)>,
        generator: Generator,
        modules: Modules,
        config: Config,
        executor: Executor,
    ) -> Self {
        Self {
            start: Instant::now(),
            command_queue: command_handler,
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
            next_player: 0,
        }
    }
}

pub struct SceneState {
    spawner: SceneSpawner,
    graph: SceneGraph,
    entities: HashMap<Key, EntityId>,
}

fn process_commands(state: &mut ServerState) {
    let _span = trace_span!("process_commands").entered();

    while let Ok((cmd, tx)) = state.command_queue.try_recv() {
        match cmd {
            Command::Server(ServerCommand::Uptime) => {
                let elapsed = state.start.elapsed();
                tx.send(format!("Uptime: {:?}", elapsed)).unwrap();
            }
            Command::Server(ServerCommand::Clients) => {
                let clients = state
                    .state
                    .conns
                    .iter()
                    .map(|conn| conn.key().addr.to_string())
                    .collect::<Vec<String>>();

                let resp = if clients.is_empty() {
                    "No clients".to_owned()
                } else {
                    clients.join("\n")
                };

                tx.send(resp).unwrap();
            }
        }
    }
}
