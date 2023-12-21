use game_macros::guest_only;

use crate::record::RecordReference;

/// Spawns a new entity.
#[guest_only]
pub fn world_entity_spawn(out: *mut u64) -> u32;

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
    component_id: *const RecordReference,
    out: *mut u32,
) -> u32;

#[guest_only]
pub fn world_entity_component_get(
    entity_id: u64,
    component_id: *const RecordReference,
    out: *mut u8,
    len: u32,
) -> u32;

#[guest_only]
pub fn world_entity_component_insert(
    entity_id: u64,
    component_id: *const RecordReference,
    ptr: *const u8,
    len: u32,
) -> u32;

#[guest_only]
pub fn world_entity_component_remove(entity_id: u64, component_id: *const RecordReference) -> u32;

#[guest_only]
pub fn world_entity_children_len(entity_id: u64) -> u32;

#[guest_only]
pub fn world_entity_children_get(entity_id: u64, ptr: *mut u64) -> u32;

#[guest_only]
pub fn world_entity_parent(entity_id: u64, ptr: *mut u64) -> u32;
