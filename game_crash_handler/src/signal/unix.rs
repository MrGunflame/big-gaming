use std::backtrace::Backtrace;
use std::ffi::c_int;
use std::fmt::Write;
use std::io;
use std::os::raw::c_void;

use game_core::logger::Logger;
use linux_raw_sys::general::{
    BUS_ADRALN, BUS_ADRERR, BUS_OBJERR, FPE_FLTDIV, FPE_FLTINV, FPE_FLTOVF, FPE_FLTSUB, FPE_FLTUND,
    FPE_INTDIV, FPE_INTOVF, ILL_BADSTK, ILL_COPROC, ILL_ILLOPC, ILL_ILLOPN, ILL_ILLTRP, ILL_PRVOPC,
    ILL_PRVREG, SEGV_ACCERR, SEGV_MAPERR, SIGBUS, SIGFPE, SIGILL, SIGSEGV,
};
use nix::libc::{sigaltstack, siginfo_t, stack_t, sysconf, _SC_PAGESIZE};
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use rustix::mm::{MapFlags, MprotectFlags, ProtFlags};

const STACK_SIZE: usize = 64 * 1024;

pub(super) unsafe fn init() {
    // Register an alternative stack for signal handlers in
    // case of a stack overflow.
    let stack = allocate_stack().unwrap();
    unsafe {
        sigaltstack(&stack, core::ptr::null_mut());
    }

    let sig_action = SigAction::new(
        SigHandler::SigAction(handler),
        // All of our handlers are fatal, i.e. once called we should
        // not return to normal operation.
        // `SA_RESETHAND` resets the signal handler and retriggers then signal
        // with the default handler (abort) after the signal was intercepted
        // by us. This means the program will still create a core dump.
        SaFlags::SA_RESETHAND | SaFlags::SA_ONSTACK | SaFlags::SA_SIGINFO,
        SigSet::empty(),
    );

    for signal in [
        Signal::SIGSEGV,
        Signal::SIGBUS,
        Signal::SIGILL,
        Signal::SIGFPE,
    ] {
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

extern "C" fn handler(signal: c_int, info: *mut siginfo_t, _: *mut c_void) {
    let info = unsafe { info.read() };

    // See https://www.man7.org/linux/man-pages/man0/signal.h.0p.html
    let (signal_str, code_str) = match signal as u32 {
        SIGSEGV => {
            let signal = stringify!(SIGSEGV);
            let code = match info.si_code as u32 {
                SEGV_MAPERR => Some(stringify!(SEGV_MAPERR)),
                SEGV_ACCERR => Some(stringify!(SEGV_ACCERR)),
                _ => None,
            };

            (signal, code)
        }
        SIGBUS => {
            let signal = stringify!(SIGBUS);
            let code = match info.si_code as u32 {
                BUS_ADRALN => Some(stringify!(BUS_ADRALN)),
                BUS_ADRERR => Some(stringify!(BUS_ADRERR)),
                BUS_OBJERR => Some(stringify!(BUS_OBJERR)),
                _ => None,
            };

            (signal, code)
        }
        SIGILL => {
            let signal = stringify!(SIGILL);
            let code = match info.si_code as u32 {
                ILL_ILLOPC => Some(stringify!(ILL_ILLOPC)),
                ILL_ILLOPN => Some(stringify!(ILL_ILLOPN)),
                ILL_ILLTRP => Some(stringify!(ILL_ILLTRP)),
                ILL_PRVOPC => Some(stringify!(ILL_PRVOPC)),
                ILL_PRVREG => Some(stringify!(ILL_PRVREG)),
                ILL_COPROC => Some(stringify!(ILL_COPROC)),
                ILL_BADSTK => Some(stringify!(ILL_BADSTK)),
                _ => None,
            };

            (signal, code)
        }
        SIGFPE => {
            let signal = stringify!(SIGFPE);
            let code = match info.si_code as u32 {
                FPE_INTDIV => Some(stringify!(FPE_INTDIV)),
                FPE_INTOVF => Some(stringify!(FPE_INTOVF)),
                FPE_FLTDIV => Some(stringify!(FPE_FLTDIV)),
                FPE_FLTOVF => Some(stringify!(FPE_FLTOVF)),
                FPE_FLTUND => Some(stringify!(FPE_FLTUND)),
                FPE_FLTINV => Some(stringify!(FPE_FLTINV)),
                FPE_FLTSUB => Some(stringify!(FPE_FLTSUB)),
                _ => None,
            };

            (signal, code)
        }
        _ => ("UNKNOWN", None),
    };

    let Some(logger) = Logger::get() else {
        return;
    };

    let mut buf = String::with_capacity(4096);
    match code_str {
        Some(code_str) => writeln!(
            buf,
            "received {} signal ({}) with code {}",
            signal_str, signal, code_str,
        )
        .ok(),
        None => writeln!(buf, "received {} signal ({})", signal_str, signal).ok(),
    };

    let backtrace = Backtrace::force_capture();
    writeln!(buf, "{}", backtrace).ok();

    logger.write(&buf);
    logger.flush();

    // Reset the handler to the default.
    if let Ok(signal) = Signal::try_from(signal) {
        let sig_action = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
        if let Err(_) = unsafe { sigaction(signal, &sig_action) } {
            // If unsetting the handler fails this handler may end
            // up being called in a infinite loop.
            // As a counter-measure we just exit "cleanly". This will not
            // execute the default signal handler again and may mean that
            // no core dump is created, but the process will never hang
            // indefinitely.
            std::process::exit(1);
        }
    }
}

/// Allocates a new stack for use in [`sigaltstack`].
fn allocate_stack() -> io::Result<stack_t> {
    let guard_page = get_page_size();

    // Allocate memory for the stack plus an extra guard page
    // before the stack as required.
    // SAFETY: The pointer is null.
    let mmap_ptr = unsafe {
        rustix::mm::mmap_anonymous(
            core::ptr::null_mut(),
            STACK_SIZE + guard_page,
            ProtFlags::empty(),
            MapFlags::PRIVATE,
        )?
    };

    let stack_ptr = (mmap_ptr as usize + guard_page) as *mut std::ffi::c_void;

    // Make the stack (without the guard page) read/writable.
    // SAFETY: The `stack_ptr` is part of the valid `mmap_ptr`.
    unsafe {
        rustix::mm::mprotect(
            stack_ptr,
            STACK_SIZE,
            MprotectFlags::READ | MprotectFlags::WRITE,
        )?;
    }

    Ok(stack_t {
        ss_sp: stack_ptr,
        ss_size: STACK_SIZE,
        ss_flags: 0,
    })
}

/// Returns the page size of the current system.
fn get_page_size() -> usize {
    unsafe { sysconf(_SC_PAGESIZE).try_into().unwrap() }
}
