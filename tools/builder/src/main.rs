use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

const BUILD_OUTPUT: &str = "build";

const FLAGS: &[&str] = &[
    // Disable loading of Vulkan validation layers.
    "render_debug_layers_disable",
    // Disable the debug render mode for UI primitives.
    "ui_debug_render_disable",
];

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Copy, Clone, Debug, Subcommand)]
enum Command {
    Build,
    Clean,
}

fn main() {
    let args = Args::parse();

    let root = project_root_path();
    let build_path = root.join(BUILD_OUTPUT);

    match args.cmd {
        Command::Build => {
            let client = root.join("game_client");
            build_cargo(client);
            move_artifact(&root, "game_client", &build_path);

            let server = root.join("game_server");
            build_cargo(server);
            move_artifact(&root, "game_server", &build_path);
        }
        Command::Clean => {
            let mut cmd = std::process::Command::new("cargo")
                .arg("clean")
                .current_dir(&root)
                .spawn()
                .unwrap();

            let status = cmd.wait().unwrap();
            assert!(status.success());

            std::fs::remove_dir_all(build_path).unwrap();
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

    let rustflags = FLAGS
        .iter()
        .map(|flag| format!("--cfg={}", flag))
        .collect::<Vec<_>>()
        .join("\x1f");

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

    if !status.success() {
        println!("build failed");
        std::process::exit(1);
    }
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
