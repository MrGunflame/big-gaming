use std::time::{Duration, Instant};

use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, IntoSystemConfig, Plugin, Quat, Query, Res, ResMut,
    Transform, Vec3,
};
use bevy_rapier3d::prelude::{Collider, Velocity};
use game_common::actors::human::Human;
use game_common::bundles::ActorBundle;
use game_common::components::combat::Health;
use game_common::components::player::Player;
use game_common::components::race::RaceId;
use game_common::entity::{Entity, EntityData, EntityId, EntityMap};
use game_common::world::entity::{Actor as WorldActor, Object as WorldObject};
use game_common::world::source::StreamingSource;
use game_net::proto::Frame;
use game_net::snapshot::{Command, CommandQueue, Snapshot};
use game_net::world::WorldState;

use crate::conn::Connections;
use crate::entity::{self, ServerEntityGenerator};

pub struct ServerPlugins;

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(ServerEntityGenerator::new())
            .insert_resource(WorldState::new())
            .insert_resource(EntityMap::default())
            .add_system(flush_command_queue)
            .add_system(update_snapshots.after(flush_command_queue));
    }
}

fn flush_command_queue(
    mut gen: Res<ServerEntityGenerator>,
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    queue: Res<CommandQueue>,
    mut entities: Query<(&Entity, &mut Transform, &mut Velocity)>,
    mut map: ResMut<EntityMap>,
    mut world: ResMut<WorldState>,
    assets: Res<AssetServer>,
) {
    while let Some(msg) = queue.pop() {
        tracing::info!("got command {:?}", msg.command);

        // Get the world state at the time the client sent the command.
        let client_time = Instant::now() - Duration::from_millis(100);
        let Some(mut view) = world.get_mut(client_time) else {
            tracing::warn!("No snapshots yet");
            return;
        };

        match msg.command {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                data,
            } => {}
            Command::EntityDestroy { id } => {
                // commands.entity(id).despawn();
            }
            Command::EntityTranslate { id, translation } => {
                let ent = map.get(id).unwrap();

                if let Ok((ent, mut transform, _)) = entities.get_mut(ent) {
                    let mut entity = view.get_mut(id).unwrap();
                    entity.transform.translation = translation;
                    // transform.translation = translation;
                } else {
                    tracing::warn!("unknown entity {:?}", ent);
                }
            }
            Command::EntityRotate { id, rotation } => {
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.rotation = rotation;
            }
            Command::EntityVelocity { id, linvel, angvel } => {
                let ent = map.get(id).unwrap();

                let (ent, _, mut velocity) = entities.get_mut(ent).unwrap();
                velocity.linvel = linvel;
                velocity.angvel = angvel;
            }
            Command::EntityHealth { id: _, health: _ } => {
                tracing::warn!("received EntityHealth from client, ignored");
            }
            Command::Connected => {
                let id = EntityId::new();

                let mut actor = ActorBundle::default();
                actor.transform.transform.translation.y += 5.0;
                actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

                let mut cmds = commands.spawn(actor);
                cmds.insert(Player)
                    .insert(StreamingSource::default())
                    .insert(Entity {
                        id,
                        transform: Transform::default(),
                        data: EntityData::Actor {
                            race: RaceId(1.into()),
                            health: Health::new(50),
                        },
                    });
                Human::default().spawn(&assets, &mut cmds);

                let ent = cmds.id();

                view.spawn(Entity {
                    id,
                    transform: Transform::default(),
                    data: EntityData::Actor {
                        race: RaceId(1.into()),
                        health: Health::new(50),
                    },
                });

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
            Command::Disconnected => {
                if let Some(id) = connections.host(msg.id) {
                    view.despawn(id);
                    let entity = map.get(id).unwrap();
                    commands.entity(entity).despawn_recursive();
                }

                // Remove the player from the connections ref.
                connections.remove(msg.id);
            }
            Command::SpawnHost { id } => (),
        }

        dbg!(&view);

        drop(view);
        world.patch_delta(client_time);
    }
}

fn update_snapshots(
    connections: Res<Connections>,
    // FIXME: Make dedicated type for all shared entities.
    // mut entities: Query<(&mut Entity, &Transform)>,
    mut world: ResMut<WorldState>,
) {
    for conn in connections.iter_mut() {
        let mut state = conn.data.state.write();

        if state.full_update {
            state.full_update = false;

            // Send full state
            // The delta from the current frame is "included" in the
            // full update.
            let Some(view) = world.front() else {
                continue;
            };

            for entity in view.iter() {
                conn.data.handle.send_cmd(Command::EntityCreate {
                    id: entity.id,
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    data: entity.data.clone(),
                });
            }
        } else {
            let delta = world.delta();

            // Send only deltas
            conn.set_delta(delta.to_vec());
        }
    }

    world.insert(Instant::now());

    // Only keep 2s.
    if world.len() > 120 {
        world.pop();
    }
}
