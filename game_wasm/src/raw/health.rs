use bytemuck::{Pod, Zeroable};

use super::PtrMut;

#[link(wasm_import_module = "host")]
extern "C" {
    pub fn health_get(entity_id: u64, out: PtrMut<Health>) -> u32;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(C)]
pub struct Health(pub u32);
