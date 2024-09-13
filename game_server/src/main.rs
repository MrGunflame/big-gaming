use std::io::{stdin, ErrorKind};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use game_core::command::{tokenize, ParseError};
use game_core::modules;
use game_server::command::Command;
use game_server::config::{Config, LoadConfigError};
use game_server::server::Server;
use game_server::ServerState;
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Parser)]
#[command(author, version, author, about, long_about = None)]
struct Args {
    /// Set the path to the config file.
    #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
    config: PathBuf,
    /// Create a new config file. Ignored if config file already exists
    #[arg(long)]
    create_config: bool,
}

fn main() -> ExitCode {
    game_core::logger::init();

    let args = Args::parse();

    let config = match Config::from_file(&args.config) {
        Ok(config) => config,
        Err(err) => match err {
            LoadConfigError::Io(err) if err.kind() == ErrorKind::NotFound && args.create_config => {
                tracing::info!(
                    "creating new config file at {}",
                    args.config.to_string_lossy()
                );

                match Config::create_default_config(&args.config) {
                    Ok(config) => config,
                    Err(err) => {
                        fatal!(
                            "failed to create config file at {}: {}",
                            args.config.to_string_lossy(),
                            err,
                        );
                    }
                }
            }
            _ => {
                fatal!(
                    "failed to load config file from {}: {}",
                    args.config.to_string_lossy(),
                    err,
                );
            }
        },
    };

    let res = modules::load_modules().unwrap();

    let (cmd_tx, cmd_rx) = mpsc::channel(8);

    let server_state = ServerState::new(cmd_rx, res.modules, config, res.executor);

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
                Err(ParseError::Empty) => {
                    println!("unknown command");
                    continue;
                }
                Err(ParseError::Msg(msg)) => {
                    println!("{}", msg);
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
        return ExitCode::FAILURE;
    }};
}
