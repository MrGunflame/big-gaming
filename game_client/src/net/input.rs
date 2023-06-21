use bevy_ecs::query::{Changed, With};
use bevy_ecs::system::{Query, ResMut};
use game_common::components::actor::ActorProperties;
use game_common::components::player::HostPlayer;
use game_common::components::transform::Transform;
use game_net::snapshot::Command;

use super::ServerConnection;

/// Send the server the new player transform.
pub fn handle_translation_changes(
    mut conn: ResMut<ServerConnection>,
    players: Query<&Transform, (With<HostPlayer>, Changed<Transform>)>,
) {
    let Ok(transform) = players.get_single() else {
        return;
    };

    let id = conn.host;

    // FIXME: We want to check first if translation/rotation actually changed.

    conn.send(Command::EntityTranslate {
        id,
        translation: transform.translation,
    });
}

pub fn handle_rotation_changes(
    mut conn: ResMut<ServerConnection>,
    players: Query<&ActorProperties, (With<HostPlayer>, Changed<ActorProperties>)>,
) {
    let Ok(props) = players.get_single() else {
        return;
    };

    let id = conn.host;

    // FIXME: We want to check first if translation/rotation actually changed.
    conn.send(Command::EntityRotate {
        id,
        rotation: props.rotation,
    });
}
