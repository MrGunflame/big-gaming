//! Conversions between Host and Guest ABIs

use std::convert::Infallible;

use bytemuck::cast;
use game_common::components::components::Components;
use game_common::components::items::{Item as HostItem, ItemId, ItemStack as HostItemStack};
use game_common::units::Mass;
use game_wasm::raw::inventory::{Item as GuestItem, ItemStack as GuestItemStack};

pub trait ToAbi {
    type Target;

    fn to_abi(&self) -> Self::Target;
}

pub trait FromAbi {
    type Target;
    type Error;

    fn from_abi(&self) -> Result<Self::Target, Self::Error>;
}

impl ToAbi for HostItemStack {
    type Target = GuestItemStack;

    fn to_abi(&self) -> Self::Target {
        GuestItemStack {
            item: GuestItem {
                id: cast(self.item.id.0),
                equipped: self.item.equipped as u8,
                hdden: self.item.hidden as u8,
                _pad0: 0,
            },
            quantity: self.quantity,
        }
    }
}

impl FromAbi for GuestItemStack {
    type Target = HostItemStack;
    type Error = Infallible;

    fn from_abi(&self) -> Result<Self::Target, Self::Error> {
        Ok(HostItemStack {
            item: HostItem {
                id: ItemId(cast(self.item.id)),
                mass: Mass::default(),
                components: Components::new(),
                equipped: false,
                hidden: false,
            },
            quantity: self.quantity,
        })
    }
}
