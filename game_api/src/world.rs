use game_common::entity::EntityId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct world_t {
    _unused: [u8; 0],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct entity_t {
    _unused: [u8; 0],
}

#[derive(Copy, Clone, Debug)]
pub struct entity_data_t {
    kind: u32,
}

pub const ENTITY_KIND_TERRAIN: u32 = 0;
pub const ENTITY_KIND_OBJECT: u32 = 1;
pub const ENTITY_KIND_ACTOR: u32 = 3;
pub const ENTITY_KIND_ITEM: u32 = 4;

pub unsafe fn entity_create(world: *mut world_t) -> *mut entity_t {
    0 as *mut _
}

pub unsafe fn entity_get(world: *mut world_t, id: EntityId) -> *mut entity_t {
    0 as *mut _
}

pub unsafe fn entity_release(world: *mut world_t, entity: *mut entity_t) {}

pub struct terrain_data_t {}
