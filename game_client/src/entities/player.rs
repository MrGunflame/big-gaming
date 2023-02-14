use bevy::{
    math::Vec3,
    prelude::{Bundle, Camera3dBundle, Component, Transform, *},
};
use game_common::components::player::{FocusedEntity, HostPlayer};
use game_common::world::source::StreamingSource;

use crate::{
    components::Rotation,
    plugins::respawn::RespawnPoint,
    ui::{Focus, FocusKind},
};

use super::actor::ActorBundle;

#[derive(Bundle)]
pub struct PlayerCharacterBundle {
    #[bundle]
    pub actor: ActorBundle,

    pub player_character: HostPlayer,
    pub focus: Focus,
    pub respawn_point: RespawnPoint,
    pub focused_entity: FocusedEntity,
    pub streaming_source: StreamingSource,
}

impl PlayerCharacterBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            player_character: HostPlayer,
            actor: ActorBundle::new(assets),
            focus: Focus {
                kind: FocusKind::World,
                changed: false,
            },
            respawn_point: RespawnPoint(Vec3::new(0.0, 0.0, 0.0)),
            focused_entity: FocusedEntity::None,
            streaming_source: StreamingSource::new(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Component)]
pub enum CameraPosition {
    #[default]
    FirstPerson,
    ThirdPerson {
        distance: f32,
    },
}

impl CameraPosition {
    #[inline]
    pub const fn is_first(self) -> bool {
        matches!(self, Self::FirstPerson)
    }

    #[inline]
    pub const fn is_third(self) -> bool {
        matches!(self, Self::ThirdPerson { distance: _ })
    }
}
