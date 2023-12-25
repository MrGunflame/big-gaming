//! Events dispatched from the game, handled by a script
//!

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

/// A item equip event.
///
/// A equip event is fired when the actor equippes an item.
///
/// # Event signature
///
/// `fn(item: `[`InventoryId`]`, actor: `[`EntityId`]`)`
///
/// # Examples
///
/// ```
/// use game_wasm::info;
/// use game_wasm::entity::EntityId;
/// use game_wasm::events::on_equip;
/// use game_wasm::inventory::InventoryId;
///
/// #[on_equip]
/// fn on_equip(item: InventoryId, actor: EntityId) {
///     info!("{:?} equipped {:?}!", actor, item);
/// }
/// ```
///
/// [`EntityId`]: crate::entity::EntityId
/// [`InventoryId`]: crate::inventory::InventoryId
pub use game_macros::wasm__event_on_equip as on_equip;

/// A item unequip event.
///
/// A unequip event is fired when the actor unequippes an item.
///
/// # Event signature
///
/// `fn(item: `[`InventoryId`]`, actor: `[`EntityId`]`)`
///
/// # Examples
///
/// ```
/// use game_wasm::info;
/// use game_wasm::entity::EntityId;
/// use game_wasm::events::on_unequip;
/// use game_wasm::inventory::InventoryId;
///
/// #[on_unequip]
/// fn on_unequip(item: InventoryId, actor: EntityId) {
///     info!("{:?} unequipped {:?}!", actor, item);
/// }
/// ```
///
/// [`EntityId`]: crate::entity::EntityId
/// [`InventoryId`]: crate::inventory::InventoryId
pub use game_macros::wasm__event_on_unequip as on_unequip;

pub use game_macros::wasm__event_on_cell_load as on_cell_load;

pub use game_macros::wasm__event_on_cell_unload as on_cell_unload;

pub use game_macros::wasm__event_on_update as on_update;
