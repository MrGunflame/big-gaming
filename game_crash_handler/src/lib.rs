mod dialog;
mod signal;
mod sys;

use std::env::current_exe;
use std::ffi::OsString;
use std::io::{self, stdin, IsTerminal};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode, Stdio, Termination};

const FORK_FLAG: &str = "__GAME_HANDLER_FORKED";

use game_core::logger::ipc::Sender;
/// Wraps the function in the crash handling harness and exports it as the `main` function.
pub use game_macros::crash_handler_main as main;
use nix::fcntl::{FcntlArg, FdFlag};

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

        let fd_str = std::env::var("MY_FD").unwrap();
        let fd: i32 = fd_str.parse().unwrap();

        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        dbg!(&fd);

        let sender = Sender::from_fd(fd);
        sender.store();

        let termination = main();
        return termination.report();
    }

    let (tx, rx) = match game_core::logger::ipc::channel() {
        Ok((tx, rx)) => (tx, rx),
        Err(err) => {
            eprintln!("failed to create pipe for process communication: {}", err);
            return ExitCode::FAILURE;
        }
    };

    match fork_main(args, tx) {
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

fn fork_main(args: Vec<OsString>, sender: Sender) -> Result<Status, io::Error> {
    let program = current_exe()?;

    let mut cmd = Command::new(program);
    cmd.env(FORK_FLAG, "");
    cmd.env("RUST_BACKTRACE", "full");
    cmd.args(args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        let fd = sender.into_fd();
        dbg!(&fd);
        dbg!("pre fork");

        cmd.env("MY_FD", fd.as_raw_fd().to_string());

        cmd.pre_exec(move || {
            nix::fcntl::fcntl(fd.as_raw_fd(), FcntlArg::F_SETFD(FdFlag::empty()));
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;

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
