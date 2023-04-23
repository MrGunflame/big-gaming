//! Patch section

use self::item::ItemPatch;

pub mod components;
pub mod item;

pub enum Patch {
    Item(ItemPatch),
}

pub enum PatchKind {
    Item,
}
