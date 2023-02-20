use bevy::prelude::{Commands, Entity, Plugin, Query, Res, ResMut, Transform};
use game_common::bundles::ActorBundle;
use game_common::components::actor::Actor;
use game_common::components::object::Object;
use game_common::components::player::Player;
use game_common::world::entity::Object as WorldObject;
use game_common::world::source::StreamingSource;
use game_net::proto::Frame;
use game_net::snapshot::{Command, CommandQueue, Snapshot};

use crate::conn::Connections;

pub struct ServerPlugins;

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(flush_command_queue)
            .add_system(update_snapshots);
    }
}

fn flush_command_queue(
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    queue: Res<CommandQueue>,
    mut entities: Query<(Entity, &mut Transform)>,
) {
    while let Some(msg) = queue.pop() {
        tracing::info!("got command {:?}", msg.command);

        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
            } => {}
            Command::EntityDestroy { id } => {
                commands.entity(id).despawn();
            }
            Command::EntityTranslate { id, translation } => {
                let (ent, mut transform) = entities.get_mut(id).unwrap();
                transform.translation = translation;
            }
            Command::EntityRotate { id, rotation } => {
                let (ent, mut transform) = entities.get_mut(id).unwrap();
                transform.rotation = rotation;
            }
            Command::PlayerJoin => {
                let id = commands
                    .spawn(ActorBundle::default())
                    .insert(Player)
                    .insert(StreamingSource::default())
                    .id();

                connections.set_host(msg.id, id);
            }
            Command::PlayerLeave => {}
            Command::SpawnHost { id } => (),
        }
    }
}

fn update_snapshots(
    mut connections: ResMut<Connections>,
    // FIXME: Make dedicated type for all shared entities.
    mut entities: Query<(Entity, &Transform, Option<&Object>, Option<&Actor>)>,
) {
    let mut snapshot = Snapshot::new();

    for (entity, transform, object, actor) in &mut entities {
        let body = match object {
            Some(obj) => WorldObject {
                id: obj.id,
                transform: *transform,
            }
            .into(),
            None => match actor {
                Some(act) => continue,
                None => continue,
            },
        };

        snapshot.update(entity, body);
    }

    for mut snap in connections.iter_mut() {
        *snap = snapshot.clone();
    }
}
