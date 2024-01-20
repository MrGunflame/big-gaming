use game_wasm::world::RecordReference;
use wasmtime::TypedFunc;

///
/// ```ignore
/// fn();
/// ```
pub type OnInit = TypedFunc<(), ()>;

/// ```ignore
/// fn(fn_ptr: *const unsafe fn(c_void), entity: EntityId);
/// ```
pub(crate) type WasmFnTrampoline = TypedFunc<(u32, u64), ()>;

#[derive(Clone, Debug)]
pub struct DispatchEvent {
    pub id: RecordReference,
    pub data: Vec<u8>,
}
