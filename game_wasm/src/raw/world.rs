use super::{Ptr, PtrMut};

#[link(wasm_import_module = "host")]
extern "C" {
    pub fn world_entity_get(id: u64, out: PtrMut<Entity>) -> u32;

    pub fn world_entity_spawn(entity: Ptr<Entity>) -> u32;

    pub fn world_entity_despawn(id: u64) -> u32;
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Entity {
    pub id: u64,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub body: EntityBody,
}

#[derive(Clone, Debug)]
#[repr(u8, C)]
pub enum EntityBody {
    Item(Item) = 0,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Item {
    pub id: u32,
}
