use std::fs::File;
use std::io::{stdin, Read};
use std::process::ExitCode;

use clap::Parser;

use game_common::world::gen::Generator;
use game_core::command::tokenize;
use game_core::modules;
use game_server::command::Command;
use game_server::config::Config;
use game_server::server::Server;
use game_server::ServerState;
use game_worldgen::gen::StaticGenerator;
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {}

fn main() -> ExitCode {
    game_core::logger::init();

    let mut config_path = std::env::current_dir().unwrap();
    config_path.push("config.toml");

    let config = match Config::from_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            fatal!("failed to load config file from {:?}: {}", config_path, err);
        }
    };

    let res = modules::load_modules();

    let generator = load_world();

    let (cmd_tx, cmd_rx) = mpsc::channel(8);

    let server_state = ServerState::new(
        cmd_rx,
        Generator::from(generator),
        res.modules,
        config,
        res.executor,
    );

    std::thread::spawn(move || {
        let stdin = stdin();
        loop {
            let mut buf = String::new();
            stdin.read_line(&mut buf).unwrap();
            let tokens = tokenize(&buf).unwrap();
            if tokens.is_empty() {
                continue;
            }

            let cmd = match Command::parse(&tokens) {
                Ok(cmd) => cmd,
                Err(_) => {
                    println!("unknown command");
                    continue;
                }
            };

            let (tx, rx) = oneshot::channel();
            cmd_tx.blocking_send((cmd, tx)).unwrap();
            let resp = rx.blocking_recv().unwrap();
            println!("{}", resp);
        }
    });

    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let conns = server_state.connections();
        let server = match Server::new(conns) {
            Ok(server) => server,
            Err(err) => {
                tracing::error!("failed to bind server: {}", err);
                return ExitCode::FAILURE;
            }
        };

        tokio::task::spawn(async move {
            if let Err(err) = server.await {
                tracing::error!("failed to run server: {}", err);
            }
        });

        server_state.run().await
    })
}

#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {{
        tracing::error!($($arg)*);
        tracing::error!("encountered fatal error, exiting");
        std::process::exit(1);
    }};
}

fn load_world() -> StaticGenerator {
    let mut file = File::open("./world.json").unwrap();

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();

    game_worldgen::from_slice(&buf).unwrap()
}
