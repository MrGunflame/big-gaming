use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use bytes::{Buf, BufMut};
use game_common::units::Mass;
use thiserror::Error;

use crate::{Decode, Encode};

use super::components::Operation;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub enum ItemPatchError {
    #[error("failed to decode flags: {0}")]
    Flags(<ItemPatchFlags as Decode>::Error),
    #[error("failed to decode mass: {0}")]
    Mass(<Mass as Decode>::Error),
    #[error("failed to decode value: {0}")]
    Value(<u64 as Decode>::Error),
    #[error("failed to decode components: {0}")]
    Components(<Vec<Operation> as Decode>::Error),
}

#[derive(Clone, Debug)]
pub struct ItemPatch {
    // inlined
    // pub flags: ItemPatchFlags,
    pub mass: Option<Mass>,
    pub value: Option<u64>,
    pub components: Option<Vec<Operation>>,
}

impl Encode for ItemPatch {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let mut flags = ItemPatchFlags::NONE;

        if self.mass.is_some() {
            flags |= ItemPatchFlags::MASS;
        }

        if self.value.is_some() {
            flags |= ItemPatchFlags::VALUE;
        }

        if self.components.is_some() {
            flags |= ItemPatchFlags::COMPONENTS;
        }

        flags.encode(&mut buf);

        if let Some(mass) = &self.mass {
            mass.encode(&mut buf);
        }

        if let Some(value) = &self.value {
            value.encode(&mut buf);
        }

        if let Some(components) = &self.components {
            components.encode(&mut buf);
        }
    }
}

impl Decode for ItemPatch {
    type Error = ItemPatchError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let flags = ItemPatchFlags::decode(&mut buf).map_err(ItemPatchError::Flags)?;

        let mut mass = None;
        let mut value = None;
        let mut components = None;

        if flags & ItemPatchFlags::MASS != ItemPatchFlags::NONE {
            mass = Some(Mass::decode(&mut buf).map_err(ItemPatchError::Mass)?);
        }

        if flags & ItemPatchFlags::VALUE != ItemPatchFlags::NONE {
            value = Some(u64::decode(&mut buf).map_err(ItemPatchError::Value)?)
        }

        if flags & ItemPatchFlags::COMPONENTS != ItemPatchFlags::NONE {
            components = Some(Vec::decode(&mut buf).map_err(ItemPatchError::Components)?);
        }

        Ok(Self {
            mass,
            value,
            components,
        })
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemPatchFlags(u8);

impl ItemPatchFlags {
    pub const NONE: Self = Self(0);

    pub const MASS: Self = Self(1);
    pub const VALUE: Self = Self(1 << 1);
    pub const COMPONENTS: Self = Self(1 << 2);
}

impl BitOr for ItemPatchFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ItemPatchFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for ItemPatchFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for ItemPatchFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl Encode for ItemPatchFlags {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        self.0.encode(buf);
    }
}

impl Decode for ItemPatchFlags {
    type Error = <u8 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u8::decode(buf).map(Self)
    }
}
