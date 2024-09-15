use std::backtrace::Backtrace;
use std::ffi::c_int;
use std::io::Write;
use std::os::raw::c_void;

use nix::libc::siginfo_t;
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};

pub(super) unsafe fn init() {
    let sig_action = SigAction::new(
        SigHandler::SigAction(handler),
        // All of our handlers are fatal, i.e. once called we should
        // not return to normal operation.
        // `SA_RESETHAND` resets the signal handler and retriggers then signal
        // with the default handler (abort) after the signal was intercepted
        // by us. This means the program will still create a core dump.
        SaFlags::SA_RESETHAND,
        SigSet::empty(),
    );

    for signal in [Signal::SIGSEGV, Signal::SIGBUS, Signal::SIGILL] {
        // This should only fail if we either
        // specify an invalid signal (which we don't)
        // or the old handler does not belong to us.
        // The caller guarantees that this is the first handler.
        // https://www.man7.org/linux/man-pages/man2/sigaction.2.html#ERRORS
        if let Err(errno) = unsafe { sigaction(signal, &sig_action) } {
            panic!("failed to install signal handler for {}: {}", signal, errno);
        }
    }
}

extern "C" fn handler(_: c_int, _info: *mut siginfo_t, _: *mut c_void) {
    let backtrace = Backtrace::force_capture();

    // Avoid the `eprintln` macro because it might panic.
    // We ignore any errors when writing to stderr because
    // there is no useful way we could handle them.
    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "Segmenation fault").ok();
    writeln!(stderr, "{}", backtrace).ok();
}
