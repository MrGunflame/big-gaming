pub fn abort() -> ! {
    unsafe {
        crate::raw::process::abort();
    }
}
