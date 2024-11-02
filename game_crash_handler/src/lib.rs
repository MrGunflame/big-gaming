mod dialog;
mod signal;

use std::env::current_exe;
use std::ffi::OsString;
use std::io::{self, stdin, IsTerminal};
use std::process::{Command, ExitCode, Stdio, Termination};

const FORK_FLAG: &str = "__GAME_HANDLER_FORKED";

/// Wraps the function in the crash handling harness and exports it as the `main` function.
pub use game_macros::crash_handler_main as main;

/// Run `main` inside the crash handling harness.
///
/// # Safety
///
/// This function must be called only once at the start of the program. When it is called all the
/// following statements must be true:
/// - The program is not (yet) multithreaded (i.e. no threads have been spawned yet).
/// - The program has not changed any signal handlers.
pub unsafe fn run<T, F>(main: F) -> ExitCode
where
    F: FnOnce() -> T,
    T: Termination,
{
    let mut enable_crash_handler = true;
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    for arg in &args {
        if arg == "--no-crash-handler" {
            enable_crash_handler = false;
        }
    }

    // To catch any form of crash inside `main` (including immediate aborts)
    // we run `main` inside a completely different process forked from our
    // main binary.

    if std::env::var_os(FORK_FLAG).is_some() || !enable_crash_handler {
        unsafe {
            // Register signal handlers for "fatal" signals like SIGSEGV.
            // This allows us to emit a backtrace before exiting.
            // SAFETY: The caller guarantees that this is called early in the program
            // lifecycle and no signal handlers have yet been changed.
            signal::init();

            // Unset the `FORK_FLAG` environment variable before running `main`.
            // This ensures that there is no observable difference compared to
            // running `main` directly.
            // SAFETY: The caller guarantees that this function is only called in a
            // single-threaded environment. `remove_var` is safe to call in single-
            // threaded environments.
            if std::env::var_os(FORK_FLAG).is_some() {
                std::env::remove_var(FORK_FLAG);
            }
        }

        // Start the profiler before the application starts.
        #[cfg(feature = "tracy")]
        game_tracing::Client::start();

        let termination = main();
        return termination.report();
    }

    // The profiler should not run on the parent process.
    #[cfg(feature = "tracy")]
    debug_assert!(!game_tracing::Client::is_running());

    match fork_main(args) {
        Ok(Status::Sucess) => {
            return ExitCode::SUCCESS;
        }
        Ok(Status::Failure) => return ExitCode::FAILURE,
        Ok(Status::Crash) => (),
        Err(err) => {
            eprintln!("Failed to fork binary: {}", err);
            return ExitCode::FAILURE;
        }
    }

    eprintln!("The game has crashed");

    if !stdin().is_terminal() {
        dialog::dialog("The game has crashed!");
    }

    ExitCode::FAILURE
}

fn fork_main(args: Vec<OsString>) -> Result<Status, io::Error> {
    let program = current_exe()?;
    let mut child = Command::new(program)
        .env(FORK_FLAG, "")
        .env("RUST_BACKTRACE", "full")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let status = child.wait()?;

    match status.code() {
        Some(0) => Ok(Status::Sucess),
        Some(1) => Ok(Status::Failure),
        // Is `None` if the process was terminated by a signal.
        // This is probably a `SIGSEGV` or similar.
        Some(_) | None => Ok(Status::Crash),
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Status {
    Sucess,
    Failure,
    Crash,
}
