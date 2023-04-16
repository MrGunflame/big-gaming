use core::panic::PanicInfo;

use crate::log::Level;

#[cfg_attr(not(test), panic_handler)]
fn panic_handler(info: &PanicInfo) -> ! {
    crate::log::log(Level::ERROR, "panic");
    panic!();
}
