use bytemuck::{Pod, Zeroable};

use super::{Ptr, PtrMut, Usize};
use crate::record::RecordReference;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "host")]
extern "C" {
    /// Returns the entity with the given `id`.
    ///
    /// # Errors
    ///
    /// - [`ERROR_NO_ENTITY`]: The entity does not exist.
    ///
    /// [`ERROR_NO_ENTITY`]: super::ERROR_NO_ENTITY
    pub fn world_entity_get(id: u64, out: PtrMut<Entity>) -> u32;

    // FIXME: Reevaluate how update functions are supposted to work.
    pub fn world_entity_set_translation(id: u64, x: f32, y: f32, z: f32) -> u32;
    pub fn world_entity_set_rotation(id: u64, x: f32, y: f32, z: f32, w: f32) -> u32;

    /// Spawns a new entity.
    pub fn world_entity_spawn(entity: Ptr<Entity>, out: PtrMut<u64>) -> u32;

    /// Despawns the entity with the given `id`.
    ///
    /// # Errors
    ///
    /// - [`ERROR_NO_ENTITY`]: The entity does not exist.
    ///
    /// [`ERROR_NO_ENTITY`]: super::ERROR_NO_ENTITY
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

    pub fn world_entity_component_remove(entity_id: u64, component_id: Ptr<RecordReference>)
        -> u32;
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_get(id: u64, out: PtrMut<Entity>) -> u32 {
    let _ = (id, out);
    panic!("`world_entity_get` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_set_translation(id: u64, x: f32, y: f32, z: f32) -> u32 {
    let _ = (id, x, y, z);
    panic!("`world_entity_set_translation` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_set_rotation(id: u64, x: f32, y: f32, z: f32, w: f32) -> u32 {
    let _ = (id, x, y, z, w);
    panic!("`world_entity_set_rotation` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_spawn(entity: Ptr<Entity>, out: PtrMut<u64>) -> u32 {
    let _ = (entity, out);
    panic!("`world_entity_spawn` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_despawn(id: u64) -> u32 {
    let _ = id;
    panic!("`world_entity_despawn` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_component_len(
    entity_id: u64,
    componnet_id: Ptr<RecordReference>,
    out: PtrMut<Usize>,
) -> u32 {
    let _ = (entity_id, componnet_id, out);
    panic!("`world_entity_component_len` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_component_get(
    entity_id: u64,
    componnet_id: Ptr<RecordReference>,
    out: PtrMut<u8>,
    len: Usize,
) -> u32 {
    let _ = (entity_id, componnet_id, out, len);
    panic!("`world_entity_component_get` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_component_insert(
    entity_id: u64,
    componnet_id: Ptr<RecordReference>,
    ptr: Ptr<u8>,
    len: Usize,
) -> u32 {
    let _ = (entity_id, componnet_id, ptr, len);
    panic!("`world_entity_component_insert` is not implemented on this target");
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn world_entity_component_remove(
    entity_id: u64,
    componnet_id: Ptr<RecordReference>,
) -> u32 {
    let _ = (entity_id, componnet_id);
    panic!("`world_entity_component_remove` is not implemented on this target");
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

// Assert that EntityBody has no padding.
const _: fn() = || {
    let _: [(); core::mem::size_of::<EntityBody>()] = [(); core::mem::size_of::<RecordReference>()];
    let _: [(); core::mem::align_of::<EntityBody>()] =
        [(); core::mem::align_of::<RecordReference>()];
    let _: [(); core::mem::size_of::<EntityBody>()] =
        [(); core::mem::size_of::<[u8; core::mem::size_of::<RecordReference>()]>()];
};

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Item {
    pub id: RecordReference,
}
