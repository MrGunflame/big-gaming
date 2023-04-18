use bytemuck::NoUninit;

use super::record::RecordReference;
use super::{Ptr, PtrMut};

#[link(wasm_import_module = "host")]
extern "C" {
    pub fn world_entity_get(id: u64, out: PtrMut<Entity>) -> u32;

    pub fn world_entity_spawn(entity: Ptr<Entity>) -> u32;

    pub fn world_entity_despawn(id: u64) -> u32;

    pub fn world_entity_component_get(
        entity_id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<Component>,
    ) -> u32;

    pub fn world_entity_component_insert(
        entity_id: u64,
        component_id: Ptr<RecordReference>,
        component: Ptr<Component>,
    ) -> u32;

    pub fn world_entity_component_remove(entity_id: u64, component_id: RecordReference) -> u32;
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
    pub id: RecordReference,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8, C)]
pub enum Component {
    I32(i32),
    I64(i64),
}

unsafe impl NoUninit for Component {}
