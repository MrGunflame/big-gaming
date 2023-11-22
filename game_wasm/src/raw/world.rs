use bytemuck::{Pod, Zeroable};
use game_macros::guest_only;

use super::{Ptr, PtrMut, Usize};
use crate::record::RecordReference;

/// Returns the entity with the given `id`.
///
/// # Errors
///
/// - [`ERROR_NO_ENTITY`]: The entity does not exist.
///
/// [`ERROR_NO_ENTITY`]: super::ERROR_NO_ENTITY
#[guest_only]
pub fn world_entity_get(entity_id: u64, out: PtrMut<Entity>) -> u32;

// FIXME: Reevaluate how update functions are supposted to work.
#[guest_only]
pub fn world_entity_set_translation(entity_id: u64, x: f32, y: f32, z: f32) -> u32;
#[guest_only]
pub fn world_entity_set_rotation(entity_id: u64, x: f32, y: f32, z: f32, w: f32) -> u32;

/// Spawns a new entity.
#[guest_only]
pub fn world_entity_spawn(entity: Ptr<Entity>, out: PtrMut<u64>) -> u32;

/// Despawns the entity with the given `id`.
///
/// # Errors
///
/// - [`ERROR_NO_ENTITY`]: The entity does not exist.
///
/// [`ERROR_NO_ENTITY`]: super::ERROR_NO_ENTITY
#[guest_only]
pub fn world_entity_despawn(id: u64) -> u32;

#[guest_only]
pub fn world_entity_component_len(
    entity_id: u64,
    component_id: Ptr<RecordReference>,
    out: PtrMut<Usize>,
) -> u32;

#[guest_only]
pub fn world_entity_component_get(
    entity_id: u64,
    component_id: Ptr<RecordReference>,
    out: PtrMut<u8>,
    len: Usize,
) -> u32;

#[guest_only]
pub fn world_entity_component_insert(
    entity_id: u64,
    component_id: Ptr<RecordReference>,
    ptr: Ptr<u8>,
    len: Usize,
) -> u32;

#[guest_only]
pub fn world_entity_component_remove(entity_id: u64, component_id: Ptr<RecordReference>) -> u32;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct Entity {
    pub id: u64,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub linvel: [f32; 3],
    pub angvel: [f32; 3],
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
