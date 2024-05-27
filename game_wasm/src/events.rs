//! Events dispatched from the game, handled by a script
//!

use crate::encoding::{encode_value, Decode, Encode};
use crate::player::PlayerId;
use crate::raw::event_dispatch;
use crate::record::{ModuleId, RecordId, RecordReference};

/// A script initialization event.
///
/// If present, the handler for this event will be called exactly before the script is being
/// used. `on_init` guaranteed to be called exactly once before any other handlers in the script
/// are being called. While `on_init` is being executed, no other handlers are being called.
///
/// # Event signature
///
/// `fn()`
///
/// # Examples
///
/// ```
/// use game_wasm::info;
/// use game_wasm::events::on_init;
///
/// #[on_init]
/// fn on_init() {
///     info!("Script was initialized!");
/// }
/// ```
pub use game_macros::wasm__event_on_init as on_init;

/// An action event.
///
/// An action event is fired when the action that references this script is called.
///
/// This event takes a different signature depending on context that it is called in.
pub use game_macros::wasm__event_on_action as on_action;

/// A collision event.
///
/// A collision event is fired when the entity collides with another entity.
///
/// # Event signature
///
/// `fn(entity: `[`EntityId`]`, other: `[`EntityId`]`)`
///
/// # Examples
///
/// ```
/// use game_wasm::info;
/// use game_wasm::entity::EntityId;
/// use game_wasm::events::on_collision;
///
/// #[on_collision]
/// fn on_collision(entity: EntityId, other: EntityId) {
///     info!("{:?} collided with {:?}!", entity, other);
/// }
/// ```
///
/// [`EntityId`]: crate::entity::EntityId
/// [`InventoryId`]: crate::inventory::InventoryId
pub use game_macros::wasm__event_on_collision as on_collision;

pub use game_macros::wasm__event_on_cell_load as on_cell_load;

pub use game_macros::wasm__event_on_cell_unload as on_cell_unload;

pub use game_macros::wasm__event_on_update as on_update;

pub trait Event: Encode + Decode {
    const ID: RecordReference;
}

pub fn dispatch_event<T>(event: &T)
where
    T: Event,
{
    dispatch_event_dynamic(T::ID, event);
}

pub fn dispatch_event_dynamic<T>(id: RecordReference, event: &T)
where
    T: Encode,
{
    let (data, fields) = encode_value(event);

    unsafe {
        event_dispatch(
            &id,
            data.as_ptr(),
            data.len(),
            fields.as_ptr(),
            fields.len(),
        );
    }
}

macro_rules! define_id {
    ($($id:ident => $val:expr),*,) => {
        $(
            pub const $id: RecordReference = RecordReference {
                module: ModuleId::CORE,
                record: RecordId($val),
            };
        )*
    };
}

define_id! {
    PLAYER_CONNECT => 0,
    PLAYER_DISCONNECT => 1,
    CELL_LOAD => 2,
    CELL_UNLOAD => 3,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
#[non_exhaustive]
pub struct PlayerConnect {
    pub player: PlayerId,
}

impl Event for PlayerConnect {
    const ID: RecordReference = PLAYER_CONNECT;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
#[non_exhaustive]
pub struct PlayerDisconnect {
    pub player: PlayerId,
}

impl Event for PlayerDisconnect {
    const ID: RecordReference = PLAYER_DISCONNECT;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CellLoad {
    pub cell: [u32; 3],
}

impl Event for CellLoad {
    const ID: RecordReference = CELL_LOAD;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CellUnload {
    pub cell: [u32; 3],
}

impl Event for CellUnload {
    const ID: RecordReference = CELL_UNLOAD;
}
