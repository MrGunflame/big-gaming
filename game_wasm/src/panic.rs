use core::panic::PanicInfo;

use crate::log::Level;
use crate::process::abort;

#[cfg_attr(not(test), panic_handler)]
fn panic_handler(info: &PanicInfo) -> ! {
    crate::log::log(Level::ERROR, "panic");
    abort();
}
