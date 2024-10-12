mod build;
mod bundle;

use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use build::{build_crate, BuildConfig};
use bundle::{Bundler, Format};
use clap::{Parser, Subcommand};

const BUILD_OUTPUT: &str = "build";

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Copy, Clone, Debug, Subcommand)]
enum Command {
    Build,
    Clean,
    Bundle,
}

fn main() -> ExitCode {
    pretty_env_logger::init();

    let args = Args::parse();

    let root = match project_root() {
        Ok(Some(root)) => root,
        Ok(None) => {
            log::error!("failed to find project root");
            return ExitCode::FAILURE;
        }
        Err(err) => {
            log::error!("failed to find project root: {}", err);
            return ExitCode::FAILURE;
        }
    };
    log::info!("using project root {}", root.to_string_lossy());

    let build_path = root.join(BUILD_OUTPUT);

    let mut stages: VecDeque<_> = match args.cmd {
        Command::Build => [Stage::Build(BuildStage {
            target: "x86_64-unknown-linux-gnu".to_owned(),
        })]
        .into(),
        Command::Bundle => [
            Stage::Build(BuildStage {
                target: "x86_64-unknown-linux-gnu".to_owned(),
            }),
            Stage::Bundle(BundleStage {
                files: vec![
                    "game_client".into(),
                    "game_server".into(),
                    "game_editor".into(),
                ],
                format: Format::TarGz,
            }),
        ]
        .into(),
        Command::Clean => [Stage::Clean].into(),
    };

    while let Some(stage) = stages.pop_front() {
        match stage {
            Stage::Build(stage) => {
                for target in ["game_client", "game_server", "game_editor"] {
                    let path = root.join(target);
                    if let Err(err) = build_crate(
                        path,
                        BuildConfig {
                            target: &stage.target,
                        },
                    ) {
                        log::error!("failed to build crate {}: {}", target, err);
                        return ExitCode::FAILURE;
                    }

                    move_artifact(&root, target, &build_path);
                }
            }
            Stage::Bundle(stage) => {
                let output_path =
                    build_path.join(format!("game.{}", stage.format.file_extension()));

                let output = match File::create(output_path) {
                    Ok(output) => output,
                    Err(err) => {
                        log::error!("failed to create bundle: {}", err);
                        return ExitCode::FAILURE;
                    }
                };

                let mut bundler = Bundler::new(stage.format, output);
                for path in stage.files {
                    let file_path = build_path.join(&path);
                    let mut file = match File::open(&file_path) {
                        Ok(file) => file,
                        Err(err) => {
                            log::error!("failed to open {}: {}", file_path.to_string_lossy(), err);
                            return ExitCode::FAILURE;
                        }
                    };

                    if let Err(err) = bundler.append(path, &mut file) {
                        log::error!("failed to append to bundle: {}", err);
                        return ExitCode::FAILURE;
                    }
                }
            }
            Stage::Clean => {
                let mut cmd = std::process::Command::new("cargo")
                    .arg("clean")
                    .current_dir(&root)
                    .spawn()
                    .unwrap();

                let status = cmd.wait().unwrap();
                assert!(status.success());

                std::fs::remove_dir_all(&build_path).unwrap();
            }
        }
    }

    ExitCode::SUCCESS
}

#[derive(Clone, Debug)]
enum Stage {
    Build(BuildStage),
    Bundle(BundleStage),
    Clean,
}

#[derive(Clone, Debug)]
struct BuildStage {
    target: String,
}

#[derive(Clone, Debug)]
struct BundleStage {
    files: Vec<PathBuf>,
    format: Format,
}

fn project_root() -> io::Result<Option<PathBuf>> {
    let mut root = std::env::current_dir()?;
    assert!(root.is_dir());

    loop {
        let git_path = root.join(".git");
        if git_path.try_exists().unwrap() {
            return Ok(Some(root));
        }

        if !root.pop() {
            break;
        }
    }

    Ok(None)
}

fn move_artifact(root: &Path, name: &str, dst: &Path) {
    let src = root.join(format!("target/x86_64-unknown-linux-gnu/release/{}", name));

    if !dst.try_exists().unwrap() {
        std::fs::create_dir_all(dst).unwrap();
    }

    println!(
        "mv {} {}",
        src.to_string_lossy(),
        dst.join(name).to_string_lossy()
    );

    std::fs::copy(src, dst.join(name)).unwrap();
}
