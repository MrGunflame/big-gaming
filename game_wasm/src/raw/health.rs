use bytemuck::{Pod, Zeroable};

use super::PtrMut;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "host")]
extern "C" {
    pub fn health_get(entity_id: u64, out: PtrMut<Health>) -> u32;
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn health_get(entity_id: u64, out: PtrMut<Health>) -> u32 {
    let _ = (entity_id, out);
    panic!("`health_get` is not implemented on this target");
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct Health(pub u32);
