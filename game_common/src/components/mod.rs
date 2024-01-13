//! All components

pub mod actions;
pub mod actor;
pub mod components;
pub mod entity;
pub mod inventory;
pub mod items;
pub mod object;
pub mod race;
pub mod terrain;
pub mod transform;

pub use game_wasm::components::builtin::*;
pub use game_wasm::components::{Component, Decode, Encode};
pub use game_wasm::player::PlayerId;
