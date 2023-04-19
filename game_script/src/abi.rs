//! Conversions between Host and Guest ABIs

use std::convert::Infallible;

use bevy_transform::prelude::Transform;
use game_common::components::components::Components;
use game_common::entity::EntityId;
use game_common::world::entity::Entity as HostEntity;
use game_common::world::entity::EntityBody;
use game_common::world::entity::EntityKind as HostEntityKind;
use game_wasm::raw::world::Entity as GuestEntity;
use game_wasm::raw::world::EntityKind as GuestEntityKind;
use glam::{Quat, Vec3};

use crate::builtin::InvalidInvariant;

pub trait ToAbi {
    type Target;

    fn to_abi(&self) -> Self::Target;
}

pub trait FromAbi {
    type Target;
    type Error;

    fn from_abi(&self) -> Result<Self::Target, Self::Error>;
}

impl ToAbi for HostEntity {
    type Target = GuestEntity;

    fn to_abi(&self) -> GuestEntity {
        GuestEntity {
            id: self.id.into_raw(),
            translation: self.transform.translation.to_array(),
            rotation: self.transform.rotation.to_array(),
            scale: self.transform.scale.to_array(),
            kind: self.body.kind().to_abi(),
            _pad0: 0,
        }
    }
}

impl ToAbi for HostEntityKind {
    type Target = GuestEntityKind;

    fn to_abi(&self) -> Self::Target {
        match self {
            Self::Terrain => GuestEntityKind::TERRAIN,
            Self::Object => GuestEntityKind::OBJECT,
            Self::Actor => GuestEntityKind::ACTOR,
            Self::Item => GuestEntityKind::ITEM,
        }
    }
}

impl FromAbi for GuestEntityKind {
    type Target = HostEntityKind;
    type Error = InvalidInvariant;

    fn from_abi(&self) -> Result<Self::Target, Self::Error> {
        match *self {
            Self::TERRAIN => Ok(HostEntityKind::Terrain),
            Self::OBJECT => Ok(HostEntityKind::Object),
            Self::ACTOR => Ok(HostEntityKind::Actor),
            Self::ITEM => Ok(HostEntityKind::Item),
            _ => Err(InvalidInvariant),
        }
    }
}

impl FromAbi for GuestEntity {
    type Target = HostEntity;
    type Error = InvalidInvariant;

    fn from_abi(&self) -> Result<Self::Target, Self::Error> {
        let body = match self.kind.from_abi()? {
            HostEntityKind::Terrain => todo!(),
            HostEntityKind::Object => todo!(),
            HostEntityKind::Actor => todo!(),
            HostEntityKind::Item => todo!(),
        };

        Ok(HostEntity {
            id: EntityId::from_raw(self.id),
            transform: Transform {
                translation: Vec3::from_array(self.translation),
                rotation: Quat::from_array(self.rotation),
                scale: Vec3::from_array(self.scale),
            },
            body,
            components: Components::new(),
        })
    }
}
