use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;

#[derive(Clone, Debug, Parser)]
struct Args {
    #[arg(short, long)]
    target_dir: PathBuf,
}

fn main() {
    pretty_env_logger::init();

    let args = Args::parse();

    let root = find_root();
    log::info!("Using project root: {}", root.to_string_lossy());

    let cmd = Command::new("cargo")
        .args(["build", "--release"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    assert!(cmd.success());

    for script in [
        Script {
            path: "mods/movement/scripts/move_forward",
            destination: "scripts/move_forward.wasm",
        },
        Script {
            path: "mods/movement/scripts/move_back",
            destination: "scripts/move_back.wasm",
        },
        Script {
            path: "mods/movement/scripts/move_left",
            destination: "scripts/move_left.wasm",
        },
        Script {
            path: "mods/movement/scripts/move_right",
            destination: "scripts/move_right.wasm",
        },
    ] {
        build_script(&root, &script);

        let mut src = root.clone();
        src.push("target/wasm32-unknown-unknown/debug");
        src.push(format!("{}.wasm", script.path.split('/').last().unwrap()));

        let mut dst = args.target_dir.clone();
        dst.push(script.destination);

        move_build_artifact(&src, &dst);
    }
}

fn build_script(root: &Path, script: &Script) {
    let opts = Options {
        target: "wasm32-unknown-unknown",
        build_std: true,
    };

    let mut path = root.to_path_buf();
    path.push(script.path);

    let res = opts
        .to_command()
        .current_dir(path)
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    assert!(res.status.success());
}

fn move_build_artifact(src: &Path, dst: &Path) {
    log::info!(
        "copy {} to {}",
        src.to_string_lossy(),
        dst.to_string_lossy()
    );

    let parent = dst.parent().unwrap();
    if !parent.try_exists().unwrap() {
        std::fs::create_dir_all(parent).unwrap();
    }

    std::fs::rename(src, dst).unwrap();
}

struct Options {
    target: &'static str,
    build_std: bool,
}

impl Options {
    fn to_command(&self) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg(&format!("--target={}", self.target));

        if self.build_std {
            cmd.arg("-Zbuild-std=core,alloc,std,panic_abort");
        }

        cmd
    }
}

struct Script {
    path: &'static str,
    destination: &'static str,
}

fn find_root() -> PathBuf {
    let output = Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;

    let path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    path.parent().unwrap().to_path_buf()
}
