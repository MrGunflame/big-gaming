use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Copy, Clone, Debug, Subcommand)]
enum Command {
    Build,
}

fn main() {
    let args = Args::parse();

    let root = project_root_path();

    match args.cmd {
        Command::Build => {
            let client = root.join("game_client");
            build_cargo(client);
            move_artifact(&root, "game_client", &root.join("build"));

            //     let server = root.join("game_server");
            //     build_cargo(server);
            //     move_artifact(&root, "game_server", &root.join("build"));
        }
    }
}

fn project_root_path() -> PathBuf {
    let mut root = std::env::current_dir().unwrap();
    assert!(root.is_dir());

    loop {
        let git_path = root.join(".git");
        if git_path.try_exists().unwrap() {
            return root;
        }

        if !root.pop() {
            break;
        }
    }

    panic!("failed to find root");
}

fn build_cargo(path: impl AsRef<Path>) {
    println!("building {}", path.as_ref().to_string_lossy());

    let rustflags =
    "--cfg=tokio_unstable\x1f--cfg=render_debug_layers_disable\x1f--cfg=ui_debug_render_disable";

    let args = [
        "+nightly",
        "build",
        "-Zbuild-std=core,alloc,std,panic_abort",
        "--target=x86_64-unknown-linux-gnu",
        "--release",
    ];

    println!("cargo {}", args.join(" "));

    let mut cmd = std::process::Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", rustflags)
        .current_dir(path)
        .args(args)
        .spawn()
        .unwrap();

    let status = cmd.wait().unwrap();
    assert!(status.success());
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
