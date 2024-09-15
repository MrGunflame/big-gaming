#[cfg(unix)]
mod unix;

pub(super) unsafe fn init() {
    #[cfg(unix)]
    unsafe {
        unix::init();
    }
}
