use bevy::prelude::{Commands, Plugin, Quat, Query, Res, ResMut, Transform, Vec3};
use bevy_rapier3d::prelude::{Collider, Velocity};
use game_common::bundles::ActorBundle;
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
            .add_system(update_snapshots);
    }
}

fn flush_command_queue(
    mut gen: Res<ServerEntityGenerator>,
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    queue: Res<CommandQueue>,
    mut entities: Query<(&Entity, &mut Transform, &mut Velocity)>,
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

                if let Ok((ent, mut transform, _)) = entities.get_mut(ent) {
                    transform.translation = translation;
                } else {
                    tracing::warn!("unknown entity {:?}", ent);
                }
            }
            Command::EntityRotate { id, rotation } => {
                let ent = map.get(id).unwrap();

                let (ent, mut transform, _) = entities.get_mut(ent).unwrap();
                transform.rotation = rotation;
            }
            Command::EntityVelocity { id, linvel, angvel } => {
                let ent = map.get(id).unwrap();

                let (ent, _, mut velocity) = entities.get_mut(ent).unwrap();
                velocity.linvel = linvel;
                velocity.angvel = angvel;
            }
            Command::PlayerJoin => {
                let id = EntityId::new();

                let mut actor = ActorBundle::default();
                actor.transform.transform.translation.y += 5.0;
                actor.physics.collider = Collider::cuboid(1.0, 1.0, 1.0);

                let ent = commands
                    .spawn(actor)
                    .insert(Player)
                    .insert(StreamingSource::default())
                    .insert(Entity {
                        id,
                        transform: Transform::default(),
                        data: EntityData::Actor {},
                    })
                    .id();

                // connections
                //     .get_mut(msg.id)
                //     .unwrap()
                //     .data
                //     .handle
                //     .send_cmd(Command::EntityCreate {
                //         id,
                //         kind: EntityKind::Actor(()),
                //         translation: Vec3::new(0.0, 1000.0, 0.0),
                //         rotation: Quat::default(),
                //     });

                map.insert(id, ent);
                connections.set_host(msg.id, id);
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
