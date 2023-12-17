//! All components

pub mod actions;
pub mod actor;
pub mod components;
pub mod entity;
pub mod inventory;
pub mod items;
pub mod object;
pub mod physics;
pub mod race;
pub mod rendering;
pub mod terrain;
pub mod transform;

use crate::module::ModuleId;
use crate::record::{RecordId, RecordReference};

macro_rules! define_id {
    ($($id:ident => $val:expr),*,) => {
        $(
            const $id: RecordReference = RecordReference {
                module: ModuleId::CORE,
                record: RecordId($val),
            };
        )*
    };
}

// Must be kept in sync with `game_wasm/src/components/builtin.rs`.
define_id! {
    TRANSFORM => 1,
    GLOBAL_TRANSFORM => 8,

    // Rendering
    MESH_INSTANCE => 2,
    DIRECTIONAL_LIGHT => 3,
    POINT_LIGHT => 4,
    SPOT_LIGHT => 5,

    // Physics
    RIGID_BODY => 6,
    COLLIDER => 7,
}

// FIXME: rename to Component
pub trait AsComponent {
    const ID: RecordReference;

    fn from_bytes(buf: &[u8]) -> Self;

    fn to_bytes(&self) -> Vec<u8>;
}
