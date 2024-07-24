pub mod command;
pub mod config;
pub mod conn;
pub mod net;
pub mod plugins;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod world;

use std::fmt::Write;
use std::time::{Duration, Instant};

use command::Command;
use game_common::events::EventQueue;
use game_common::world::gen::Generator;
use game_core::command::{GameCommand, ServerCommand};
use game_core::counter::{Interval, UpdateCounter};
use game_core::modules::Modules;
use game_script::Executor;
use game_tasks::TaskPool;
use server::ConnectionPool;
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
        let conns = ConnectionPool::new(state);
        tokio::task::spawn(async move {
            let server = match Server::new(conns) {
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

        state.state.control_frame.inc();
        let cf = state.state.control_frame.get();

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
            next_player: 0,
        }
    }

    pub fn connections(&self) -> ConnectionPool {
        ConnectionPool::new(self.state.clone())
    }

    /// Starts the server systems.
    ///
    /// Note that this function never returns but it is safe to interrupt at yield points.
    pub async fn run(mut self) -> ! {
        let timestep = Duration::from_secs(1) / self.state.config.timestep;
        let mut interval = Interval::new(timestep);

        let mut ups = UpdateCounter::new();
        loop {
            let _span = trace_span!("update").entered();

            let now = Instant::now();
            interval.wait(now).await;

            process_commands(&mut self);
            tick(&mut self);

            self.state.control_frame.inc();
            let cf = self.state.control_frame.get();

            ups.update();

            tracing::debug!("Stepping Control frame to {:?} (UPS = {})", cf, ups.ups());
        }
    }
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
            Command::Game(GameCommand::Get(entity)) => {
                if !state.world.world.contains(entity) {
                    tx.send("invalid entity".to_owned()).unwrap();
                } else {
                    let mut resp = format!("Entity({})\n", entity.into_raw());

                    for (id, _) in state.world.world.components(entity).iter() {
                        let mut name = "Unknown";

                        if let Some(module) = state.modules.get(id.module) {
                            if let Some(record) = module.records.get(id.record) {
                                name = &record.name;
                            }
                        }

                        writeln!(resp, "{} ({})", name, id).unwrap();
                    }

                    tx.send(resp).unwrap();
                }
            }
            Command::Empty => {}
        }
    }
}
