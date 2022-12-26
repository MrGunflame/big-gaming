use bevy::{
    math::Vec3,
    prelude::{Bundle, Camera3dBundle, Component, Transform, *},
};
use common::components::player::FocusedEntity;

use crate::{components::Rotation, plugins::respawn::RespawnPoint, ui::Focus};

use super::actor::ActorBundle;

#[derive(Debug, Component)]
pub struct PlayerCharacter;

#[derive(Bundle)]
pub struct PlayerCharacterBundle {
    #[bundle]
    pub actor: ActorBundle,

    pub player_character: PlayerCharacter,
    pub focus: Focus,
    pub respawn_point: RespawnPoint,
    pub focused_entity: FocusedEntity,
}

impl PlayerCharacterBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            player_character: PlayerCharacter,
            actor: ActorBundle::new(assets),
            focus: Focus::World,
            respawn_point: RespawnPoint(Vec3::new(0.0, 0.0, 0.0)),
            focused_entity: FocusedEntity::None,
        }
    }
}

#[derive(Bundle)]
pub struct PlayerCameraBundle {
    #[bundle]
    pub camera: Camera3dBundle,
    pub rotation: Rotation,
    pub camera_position: CameraPosition,
}

impl PlayerCameraBundle {
    pub fn new() -> Self {
        Self {
            camera: Camera3dBundle {
                transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::ZERO),
                ..Default::default()
            },
            rotation: Rotation::new(),
            camera_position: CameraPosition::FirstPerson,
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
