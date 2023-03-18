use std::time::{Duration, Instant};

use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, IntoSystemConfig, Plugin, Query, Res, ResMut,
    Transform, Vec3,
};
use bevy_rapier3d::prelude::Velocity;
use game_common::actors::human::Human;
use game_common::bundles::ActorBundle;
use game_common::components::combat::Health;
use game_common::components::player::Player;
use game_common::components::race::RaceId;
use game_common::entity::{Entity, EntityData, EntityId, EntityMap};
use game_common::world::source::StreamingSource;
use game_common::world::CellId;
use game_net::snapshot::{Command, CommandQueue, EntityChange};
use game_net::world::{CellViewRef, WorldState};

use crate::conn::Connections;
use crate::entity::ServerEntityGenerator;

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
    mut commands: Commands,
    connections: Res<Connections>,
    queue: Res<CommandQueue>,
    mut entities: Query<(&Entity, &mut Transform, &mut Velocity)>,
    map: Res<EntityMap>,
    mut world: ResMut<WorldState>,
    assets: Res<AssetServer>,
) {
    while let Some(msg) = queue.pop() {
        tracing::trace!("got command {:?}", msg.command);

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
                    transform: Transform::from_translation(Vec3::new(0.0, 32.0, 0.0)),
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

                let conn = connections.get_mut(msg.id).unwrap();
                let mut state = conn.data.state.write();
                state.id = Some(id);
                state.cells = vec![CellId::new(0.0, 0.0, 0.0)];

                tracing::info!("spawning host {:?} in cell", msg.id);
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
    let Some(prev) = world.at(world.len().wrapping_sub(2)) else {
        world.insert(Instant::now());
        return;
    };

    let Some(curr) = world.at(world.len().wrapping_sub(1)) else {
        world.insert(Instant::now());
        return;
    };

    for conn in connections.iter_mut() {
        let mut state = conn.data.state.write();

        let Some(id) = state.id else {
            continue
        };

        if state.full_update {
            state.full_update = false;

            // Send full state
            // The delta from the current frame is "included" in the
            // full update.

            let host = curr.get(id).unwrap();
            let cell = curr.cell(host.transform.translation.into()).unwrap();

            for entity in cell.iter() {
                conn.data.handle.send_cmd(Command::EntityCreate {
                    id: entity.id,
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    data: entity.data.clone(),
                });
            }
        } else {
            // let mut changes = world.delta().to_vec();
            let mut changes = Vec::new();

            let host = curr.get(id).unwrap();

            let cell_id = CellId::from(host.transform.translation);
            // Host changed cells
            if !state.cells.contains(&cell_id) {
                tracing::info!("Moving host from {:?} to {:?}", state.cells, cell_id);

                // Host changed cells
                let unload = state.cells.clone();

                state.cells.clear();

                state.cells.push(host.transform.translation.into());

                // Destroy all entities in unloaded cells.
                for id in unload {
                    let cell = curr.cell(id).unwrap();

                    // Destroy all entities (for the client) from the unloaded cell.
                    for entity in cell.iter() {
                        changes.push(EntityChange::Destroy { id: entity.id });
                        dbg!("destroy {:?}", entity);
                    }
                }

                // Create all entities in loaded cells.
                dbg!(&cell_id);
                dbg!(&curr);
                let cell = curr.cell(cell_id).unwrap();

                for entity in cell.iter() {
                    // Don't duplicate the player actor.
                    if entity.id == host.id {
                        continue;
                    }

                    changes.push(EntityChange::Create {
                        id: entity.id,
                        data: entity.clone(),
                    });
                    dbg!("c");
                }

                // Host in same cell
            } else {
                let prev_cell = prev.cell(cell_id).unwrap();
                let curr_cell = curr.cell(cell_id).unwrap();

                changes.extend(CellViewRef::delta(Some(prev_cell), curr_cell));
            }

            conn.set_delta(changes);
        }
    }

    world.insert(Instant::now());

    // Only keep 2s.
    if world.len() > 120 {
        world.pop();
    }
}
