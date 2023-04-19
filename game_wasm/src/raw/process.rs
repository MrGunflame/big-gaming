#[link(wasm_import_module = "host")]
extern "C" {
    pub fn abort() -> !;
}
