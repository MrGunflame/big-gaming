use std::time::{Duration, Instant};

use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, Plugin, Res, ResMut, Transform, Vec3,
};
use game_common::actors::human::Human;
use game_common::bundles::ActorBundle;
use game_common::components::combat::Health;
use game_common::components::components::Components;
use game_common::components::player::Player;
use game_common::components::race::RaceId;
use game_common::entity::{EntityId, EntityMap};
use game_common::events::{ActionEvent, EntityEvent, Event, EventQueue};
use game_common::world::entity::{Actor, Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::source::{StreamingSource, StreamingSources, StreamingState};
use game_common::world::world::WorldState;
use game_common::world::CellId;
use game_net::conn::ConnectionId;
use game_net::snapshot::{Command, CommandQueue, ConnectionMessage, Response, Status};
use game_script::scripts::Scripts;
use game_script::ScriptServer;

use crate::conn::{Connection, Connections};
use crate::entity::ServerEntityGenerator;
use crate::net::state::Cells;
use crate::world::level::Level;

pub struct ServerPlugins;

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(ServerEntityGenerator::new());
        app.insert_resource(WorldState::new());
        app.insert_resource(EntityMap::default());

        app.add_system(tick);
    }
}

// All systems need to run sequentially.
pub fn tick(
    commands: Commands,
    conns: Res<Connections>,
    mut world: ResMut<WorldState>,
    queue: Res<CommandQueue>,
    map: Res<EntityMap>,
    assets: Res<AssetServer>,
    level: Res<Level>,
    mut sources: ResMut<StreamingSources>,
    mut pipeline: ResMut<game_physics::Pipeline>,
    mut event_queue: ResMut<EventQueue>,
    server: Res<ScriptServer>,
    scripts: Res<Scripts>,
) {
    update_client_heads(&conns, &mut world);
    flush_command_queue(
        commands,
        &conns,
        &queue,
        &map,
        &mut world,
        &assets,
        &mut event_queue,
    );

    crate::world::level::update_streaming_sources(&mut sources, &world);
    crate::world::level::update_level(&sources, &level, &mut world);

    pipeline.step(&mut world, &mut event_queue);

    game_script::plugin::flush_event_queue(&mut event_queue, &mut world, &server, &scripts);

    // Push snapshots last always
    update_snapshots(&conns, &world);
}

fn update_client_heads(conns: &Connections, world: &mut WorldState) {
    world.insert(Instant::now());

    for conn in conns.iter() {
        let old_head = conn.state().write().head;

        let client_time = Instant::now() - Duration::from_millis(100);
        let head = world.index(client_time).unwrap_or(world.len() - 1);

        // assert_ne!(old_head, head);

        conn.state().write().head = head;
    }

    if world.len() > 120 {
        world.pop();
    }
}

fn flush_command_queue(
    mut commands: Commands,
    connections: &Connections,
    queue: &CommandQueue,
    map: &EntityMap,
    world: &mut WorldState,
    assets: &AssetServer,
    events: &mut EventQueue,
) {
    while let Some(msg) = queue.pop() {
        tracing::trace!("got command {:?}", msg.command);

        let conn = connections.get(msg.conn).unwrap();
        let head = conn.state().read().head;

        // Get the world state at the time the client sent the command.
        // let Some(mut view) = world.at_mut(head) else {
        let Some(mut view) = world.front_mut() else {
            tracing::warn!("No snapshots yet");
            return;
        };

        if let Some(id) = msg.id {
            conn.push_proc_msg(id);
        }

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
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.translation = translation;
            }
            Command::EntityRotate { id, rotation } => {
                let mut entity = view.get_mut(id).unwrap();
                entity.transform.rotation = rotation;
            }
            Command::EntityVelocity { id, linvel, angvel } => {
                let ent = map.get(id).unwrap();

                // let (ent, _, mut velocity) = entities.get_mut(ent).unwrap();
                // velocity.linvel = linvel;
                // velocity.angvel = angvel;
            }
            Command::EntityHealth { id: _, health: _ } => {
                tracing::warn!("received EntityHealth from client, ignored");
            }
            Command::EntityAction { id, action } => {
                events.push(EntityEvent {
                    entity: id,
                    event: Event::Action(ActionEvent {
                        entity: id,
                        invoker: id,
                        action,
                    }),
                });
            }
            Command::Connected => {
                let id = view.spawn(Entity {
                    id: EntityId::dangling(),
                    transform: Transform::from_translation(Vec3::new(10.0, 32.0, 10.0)),
                    body: EntityBody::Actor(Actor {
                        race: RaceId(1.into()),
                        health: Health::new(50),
                    }),
                    components: Components::new(),
                });

                view.upate_streaming_source(id, StreamingState::Create);

                let mut actor = ActorBundle::default();
                actor.transform.transform.translation.y += 5.0;
                actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

                let mut cmds = commands.spawn(actor);
                cmds.insert(Player)
                    .insert(StreamingSource::default())
                    .insert(Entity {
                        id,
                        transform: Transform::default(),
                        body: EntityBody::Actor(Actor {
                            race: RaceId(1.into()),
                            health: Health::new(50),
                        }),
                        components: Components::new(),
                    });
                Human::default().spawn(&assets, &mut cmds);

                let ent = cmds.id();

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
                // FIXME: This should not be set in this snapshot, but in the most
                // recent one.
                conn.set_host(id, view.creation());

                let mut state = conn.state().write();
                state.id = Some(id);
                state.cells = Cells::new(CellId::new(0.0, 0.0, 0.0));

                tracing::info!("spawning host {:?} in cell", msg.id);
            }
            Command::Disconnected => {
                if let Some(id) = conn.host() {
                    view.despawn(id);
                    let entity = map.get(id).unwrap();
                    commands.entity(entity).despawn_recursive();
                }

                // Remove the player from the connections ref.
                connections.remove(msg.conn);
            }
            Command::SpawnHost { id } => (),
            Command::ReceivedCommands { ids: _ } => (),
        }

        drop(view);
    }
}

fn update_snapshots(
    connections: &Connections,
    // FIXME: Make dedicated type for all shared entities.
    // mut entities: Query<(&mut Entity, &Transform)>,
    world: &WorldState,
) {
    for conn in connections.iter() {
        update_client(&conn, world);
    }
}

fn update_client(conn: &Connection, world: &WorldState) {
    let mut state = conn.state().write();

    let Some(id) = state.id else {
        return;
    };

    // let Some(prev) = world.at(state.head.saturating_sub(1)) else {
    //     return;
    // };

    let Some(curr) = world.front() else {
        return;
    };

    // Send full state
    // The delta from the current frame is "included" in the full update.
    if state.full_update {
        state.full_update = false;

        let host = curr.get(id).unwrap();
        let cell = curr.cell(host.transform.translation.into());

        for entity in cell.iter() {
            conn.handle().send_cmd(ConnectionMessage {
                id: None,
                conn: ConnectionId(0),
                snapshot: curr.creation(),
                command: Command::EntityCreate {
                    id: entity.id,
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    data: entity.body.clone(),
                },
            });
        }

        return;
    }

    let mut changes = Vec::new();

    let host = curr.get(id).unwrap();

    let cell_id = CellId::from(host.transform.translation);
    // Host changed cells
    if !state.cells.contains(cell_id) {
        tracing::info!("Moving host from {:?} to {:?}", state.cells, cell_id);

        // Host changed cells
        let update = state.cells.set(host.transform.translation.into());

        // Destroy all entities in unloaded cells.
        for id in update.unloaded() {
            let cell = curr.cell(id);

            // Destroy all entities (for the client) from the unloaded cell.
            for entity in cell.iter() {
                debug_assert_ne!(entity.id, host.id);

                changes.push(EntityChange::Destroy { id: entity.id });
            }
        }

        // Create all entities in loaded cells.
        for cell_id in update.loaded() {
            let cell = curr.cell(cell_id);

            for entity in cell.iter() {
                // Don't duplicate the player actor.
                if entity.id == host.id {
                    continue;
                }

                changes.push(EntityChange::Create {
                    entity: entity.clone(),
                });
            }
        }

        // Host in same cell
    } else {
        // let prev_cell = prev.cell(cell_id);
        let curr_cell = curr.cell(cell_id);

        changes.extend(
            curr_cell
                .deltas()
                .iter()
                .cloned()
                .map(|d| match &d {
                    EntityChange::Translate {
                        id,
                        translation: _,
                        cell,
                    } => {
                        // Note that CellIds returned from translation close to borders
                        // are not well-defined.
                        // This is a fix that would cause the host to get destroyed and
                        // recreated in the same tick.
                        // FIXME: This still doesn't seem like a good solution.
                        if *id == host.id {
                            return d;
                        }

                        if let Some(cell) = cell {
                            // The cell that the entity moved into is not loaded by the
                            // client. Remove the entity from the client view.
                            if !state.cells.contains(cell.to) {
                                EntityChange::Destroy { id: *id }
                            // The cell that the entity moved from was not loaded by the
                            // client. Add the entity to the client view.
                            } else if !state.cells.contains(cell.from) {
                                let entity = curr.get(*id).unwrap();

                                EntityChange::Create {
                                    entity: entity.clone(),
                                }
                            } else {
                                d
                            }
                        } else {
                            d
                        }
                    }
                    _ => d,
                })
                .collect::<Vec<_>>(),
        );
    }

    // The host should never be destroyed.
    if cfg!(debug_assertions) {
        for event in &changes {
            match event {
                EntityChange::Destroy { id } => {
                    assert_ne!(*id, host.id);
                }
                _ => (),
            }
        }
    }

    conn.push(changes, curr.creation());

    // Acknowledge client commands.
    let ids = conn.take_proc_msg();
    if !ids.is_empty() {
        conn.handle().send_cmd(ConnectionMessage {
            id: None,
            conn: conn.id(),
            snapshot: Instant::now(),
            command: Command::ReceivedCommands {
                ids: ids
                    .into_iter()
                    .map(|id| Response {
                        id,
                        status: Status::Received,
                    })
                    .collect(),
            },
        });
    }
}

#[cfg(test)]
mod tests {}
