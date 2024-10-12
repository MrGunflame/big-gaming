use std::io;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

const FLAGS: &[&str] = &[
    // Disable loading of Vulkan validation layers.
    "render_debug_layers_disable",
    // Disable the debug render mode for UI primitives.
    "ui_debug_render_disable",
];

#[derive(Clone, Debug)]
pub struct BuildConfig<'a> {
    pub target: &'a str,
}

pub(crate) fn build_crate<P>(path: P, config: BuildConfig<'_>) -> Result<(), BuildError>
where
    P: AsRef<Path>,
{
    log::info!("building {}", path.as_ref().to_string_lossy());

    let rustflags = FLAGS
        .iter()
        .map(|flag| format!("--cfg={}", flag))
        .collect::<Vec<_>>()
        .join("\x1f");

    let args = [
        "+nightly",
        "build",
        "-Zbuild-std=core,alloc,std,panic_abort",
        &format!("--target={}", config.target),
        "--release",
    ];

    log::info!("cargo {}", args.join(" "));

    let mut cmd = Command::new("cargo")
        .env("CARGO_ENCODED_RUSTFLAGS", rustflags)
        .current_dir(path)
        .args(args)
        .spawn()
        .map_err(BuildError::Io)?;

    let status = cmd.wait().map_err(BuildError::Io)?;

    if !status.success() {
        let code = status.code().unwrap();
        Err(BuildError::ErrorCode(code))
    } else {
        Ok(())
    }
}

#[derive(Debug, Error)]
pub(crate) enum BuildError {
    #[error(transparent)]
    Io(io::Error),
    #[error("failure status code: {0}")]
    ErrorCode(i32),
}
