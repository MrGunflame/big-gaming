use bevy::prelude::{Commands, Plugin, Quat, Query, Res, ResMut, Transform, Vec3};
use game_common::bundles::ActorBundle;
use game_common::components::actor::Actor;
use game_common::components::object::Object;
use game_common::components::player::Player;
use game_common::entity::{Entity, EntityData, EntityId, EntityMap};
use game_common::world::entity::{Actor as WorldActor, Object as WorldObject};
use game_common::world::source::StreamingSource;
use game_net::proto::{EntityKind, Frame};
use game_net::snapshot::{Command, CommandQueue, Snapshot};

use crate::conn::Connections;
use crate::entity::ServerEntityGenerator;

pub struct ServerPlugins;

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(ServerEntityGenerator::new())
            .insert_resource(EntityMap::default())
            .add_system(flush_command_queue)
            .add_system(update_snapshots)
            .add_system(mov_ent);
    }
}

fn flush_command_queue(
    mut gen: Res<ServerEntityGenerator>,
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    queue: Res<CommandQueue>,
    mut entities: Query<(&Entity, &mut Transform)>,
    mut map: ResMut<EntityMap>,
) {
    while let Some(msg) = queue.pop() {
        tracing::info!("got command {:?}", msg.command);

        match msg.command {
            Command::EntityCreate {
                kind,
                id,
                translation,
                rotation,
            } => {}
            Command::EntityDestroy { id } => {
                // commands.entity(id).despawn();
            }
            Command::EntityTranslate { id, translation } => {
                let ent = map.get(id).unwrap();

                let (ent, mut transform) = entities.get_mut(ent).unwrap();
                transform.translation = translation;
            }
            Command::EntityRotate { id, rotation } => {
                let ent = map.get(id).unwrap();

                let (ent, mut transform) = entities.get_mut(ent).unwrap();
                transform.rotation = rotation;
            }
            Command::PlayerJoin => {
                let id = EntityId::new();

                let ent = commands
                    .spawn(ActorBundle::default())
                    .insert(Player)
                    .insert(StreamingSource::default())
                    .insert(Entity {
                        id,
                        transform: Transform::default(),
                        data: EntityData::Actor {},
                    })
                    .id();

                connections
                    .get_mut(msg.id)
                    .unwrap()
                    .data
                    .handle
                    .send_cmd(Command::EntityCreate {
                        id,
                        kind: EntityKind::Actor(()),
                        translation: Vec3::default(),
                        rotation: Quat::default(),
                    });

                connections.set_host(msg.id, id);
                map.insert(id, ent);
            }
            Command::PlayerLeave => {}
            Command::SpawnHost { id } => (),
        }
    }
}

fn update_snapshots(
    connections: Res<Connections>,
    // FIXME: Make dedicated type for all shared entities.
    mut entities: Query<(&mut Entity, &Transform)>,
) {
    let mut snapshot = Snapshot::new();

    for (mut entity, transform) in &mut entities {
        // let body = match object {
        //     Some(obj) => WorldObject {
        //         id: obj.id,
        //         transform: *transform,
        //     }
        //     .into(),
        //     None => match actor {
        //         Some(act) => WorldActor {
        //             id: 0,
        //             transform: *transform,
        //         }
        //         .into(),
        //         None => continue,
        //     },
        // };

        entity.transform = *transform;

        snapshot.update(entity.clone());
    }

    for mut snap in connections.iter_mut() {
        *snap = snapshot.clone();
    }
}

fn mov_ent(mut entities: Query<(&mut Entity, &mut Transform)>) {
    for (ent, mut transf) in &mut entities {
        transf.translation.x += 0.1;
    }
}
