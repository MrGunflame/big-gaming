use bytemuck::{Pod, Zeroable};

use super::record::RecordReference;
use super::{Ptr, PtrMut, Usize};

#[link(wasm_import_module = "host")]
extern "C" {
    pub fn world_entity_get(id: u64, out: PtrMut<Entity>) -> u32;

    pub fn world_entity_spawn(entity: Ptr<Entity>, out: PtrMut<u64>) -> u32;

    pub fn world_entity_despawn(id: u64) -> u32;

    pub fn world_entity_component_len(
        entity_id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<Usize>,
    ) -> u32;

    pub fn world_entity_component_get(
        entity_id: u64,
        component_id: Ptr<RecordReference>,
        out: PtrMut<u8>,
        len: Usize,
    ) -> u32;

    pub fn world_entity_component_insert(
        entity_id: u64,
        component_id: Ptr<RecordReference>,
        ptr: Ptr<u8>,
        len: Usize,
    ) -> u32;

    pub fn world_entity_component_remove(entity_id: u64, component_id: RecordReference) -> u32;
}

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct Entity {
    pub id: u64,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub kind: EntityKind,
    pub body: EntityBody,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct EntityKind(u32);

impl EntityKind {
    pub const TERRAIN: Self = Self(1);
    pub const OBJECT: Self = Self(2);
    pub const ACTOR: Self = Self(3);
    pub const ITEM: Self = Self(4);
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union EntityBody {
    /// Unimplemented, padding
    pub terrain: [u8; core::mem::size_of::<RecordReference>()],
    pub object: RecordReference,
    pub actor: [u8; core::mem::size_of::<RecordReference>()],
    pub item: RecordReference,
}

unsafe impl Zeroable for EntityBody {}
unsafe impl Pod for EntityBody {}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Item {
    pub id: RecordReference,
}

#[cfg(test)]
mod tests {
    use crate::raw::record::RecordReference;

    use super::EntityBody;

    #[test]
    fn assert_entity_body_size_align() {
        assert_eq!(
            core::mem::size_of::<RecordReference>(),
            core::mem::size_of::<EntityBody>()
        );

        assert_eq!(
            core::mem::align_of::<RecordReference>(),
            core::mem::align_of::<EntityBody>()
        );
    }
}
