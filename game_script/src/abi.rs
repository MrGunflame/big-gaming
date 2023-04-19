//! Conversions between Host and Guest ABIs

use std::convert::Infallible;

use bevy_transform::prelude::Transform;
use game_common::entity::EntityId;
use game_common::world::entity::Entity as HostEntity;
use game_common::world::entity::EntityKind as HostEntityKind;
use game_wasm::raw::world::Entity as GuestEntity;
use game_wasm::raw::world::EntityKind as GuestEntityKind;
use glam::{Quat, Vec3};

use crate::builtin::InvalidInvariant;

pub trait ToAbi {
    type Target;

    fn to_abi(&self) -> Self::Target;
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
