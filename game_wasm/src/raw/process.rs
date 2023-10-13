#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "host")]
extern "C" {
    pub fn abort() -> !;
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn abort() -> ! {
    panic!("`abort` is not implemented on this target");
}
