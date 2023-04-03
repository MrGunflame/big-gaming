use std::time::{Duration, Instant};

use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, Query, Res, ResMut, Transform, Vec3,
};
use bevy::transform::TransformBundle;
use game_common::actors::human::Human;
use game_common::bundles::{ActorBundle, ObjectBundle};
use game_common::components::actor::ActorProperties;
use game_common::components::combat::Health;
use game_common::components::entity::InterpolateTranslation;
use game_common::components::items::LoadItem;
use game_common::components::player::HostPlayer;
use game_common::components::terrain::LoadTerrain;
use game_common::entity::EntityMap;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::source::StreamingSource;
use game_common::world::world::{WorldState, WorldViewRef};
use game_net::backlog::Backlog;
use game_net::snapshot::DeltaQueue;

use crate::bundles::VisibilityBundle;

use super::ServerConnection;

pub fn apply_world_delta(
    mut world: ResMut<WorldState>,
    mut queue: ResMut<DeltaQueue>,
    conn: Res<ServerConnection>,
) {
    let mut period = conn.interpolation_period().write();

    // Don't start a new period until the previous ended.
    if period.end > Instant::now() - Duration::from_millis(100) {
        return;
    }

    if world.len() < 2 {
        return;
    }

    // Apply client-side prediction
    let view = world.at_mut(0).unwrap();
    conn.overrides().read().apply(view);
    // drop(view);

    let (Some(curr), Some(next)) = (world.at(0), world.at(1)) else {
        return;
    };

    debug_assert_ne!(curr.creation(), next.creation());

    // The end of the previous snapshot should be the current snapshot.
    if cfg!(debug_assertions) {
        // Ignore the start, where start == end.
        if period.start != period.end {
            assert_eq!(period.end, curr.creation());
        }
    }

    period.start = curr.creation();
    period.end = next.creation();

    let delta = WorldViewRef::delta(Some(curr), next);

    for change in delta {
        queue.push(change);
    }

    world.pop();
}

pub fn flush_delta_queue(
    mut commands: Commands,
    mut queue: ResMut<DeltaQueue>,
    mut entities: Query<(
        bevy::ecs::entity::Entity,
        &mut Transform,
        Option<&mut Health>,
        Option<&mut ActorProperties>,
    )>,
    map: Res<EntityMap>,
    assets: Res<AssetServer>,
    mut backlog: ResMut<Backlog>,
    conn: Res<ServerConnection>,
) {
    // Since events are received in batches, and commands are not applied until
    // the system is done, we buffer all created entities so we can modify them
    // in place within the same batch before they are spawned into the world.
    let mut buffer: Vec<DelayedEntity> = vec![];

    // Drop the lock ASAP.
    let period = { *conn.interpolation_period().read() };

    while let Some(change) = queue.pop() {
        match change {
            EntityChange::Create { entity } => {
                tracing::info!("spawning entity {:?}", entity.id);

                buffer.push(entity.into());
            }
            EntityChange::Destroy { id } => {
                let Some(entity) = map.get(id) else {
                    tracing::warn!("attempted to destroy a non-existent entity: {:?}", id);
                    continue;
                };

                tracing::info!("despawning entity {:?}", id);

                commands.entity(entity).despawn_recursive();
            }
            EntityChange::Translate {
                id,
                translation,
                cell,
            } => {
                let Some(entity) = map.get(id) else {
                    if let Some(entity) = buffer.iter_mut().find(|e|e.entity.id==id) {
                        entity.entity.transform.translation = translation;
                    } else {
                        backlog.push(id, EntityChange::Translate { id, translation, cell });
                    }

                    continue;
                };

                if let Ok((ent, mut transform, _, _)) = entities.get_mut(entity) {
                    commands.entity(ent).insert(InterpolateTranslation {
                        src: transform.translation,
                        dst: translation,
                        start: period.start,
                        end: period.end,
                    });

                    // transform.translation = translation;
                } else {
                    tracing::warn!("attempted to translate unknown entity {:?}", id);
                }
            }
            EntityChange::Rotate { id, rotation } => {
                let Some(entity) = map.get(id) else {
                    if let Some(entity) = buffer.iter_mut().find(|e| e.entity.id == id) {
                        entity.entity.transform.rotation = rotation;
                    } else {
                        backlog.push(id, EntityChange::Rotate { id, rotation });
                    }

                    continue;
                };

                if let Ok((ent, mut transform, _, props)) = entities.get_mut(entity) {
                    if let Some(mut props) = props {
                        // Actor
                        props.rotation = rotation;
                    } else {
                        // Object
                        transform.rotation = rotation;
                    }
                } else {
                    tracing::warn!("attempted to rotate unknown entity {:?}", id);
                }
            }
            EntityChange::CreateHost { id } => {
                let Some(entity) = map.get(id) else {
                    if let Some(entity) = buffer.iter_mut().find(|e| e.entity.id == id) {
                        entity.host = true;
                    } else {
                        backlog.push(id, EntityChange::CreateHost { id });
                    }

                    continue;
                };

                commands
                    .entity(entity)
                    .insert(HostPlayer)
                    .insert(StreamingSource::new());
            }
            EntityChange::DestroyHost { id } => {
                let Some(entity) = map.get(id) else {
                    if let Some(entity) = buffer.iter_mut().find(|e| e.entity.id == id) {
                        entity.host = false;
                    } else {
                        backlog.push(id, EntityChange::DestroyHost { id });
                    }

                    continue;
                };

                commands
                    .entity(entity)
                    .remove::<HostPlayer>()
                    .remove::<StreamingSource>();
            }
            EntityChange::Health { id, health } => {
                let Some(entity) = map.get(id) else {
                    if let Some(entity) = buffer.iter_mut().find(|e| e.entity.id == id ) {
                        if let EntityBody::Actor(actor) = &mut entity.entity.body {
                            actor.health = health;
                        }
                    } else {
                        backlog.push(id, EntityChange::Health { id, health });
                    }

                    continue;
                };

                let (_, _, h, _) = entities.get_mut(entity).unwrap();
                if let Some(mut h) = h {
                    *h = health;
                } else {
                    tracing::warn!("tried to apply health to a non-actor entity");
                }
            }
            EntityChange::UpdateStreamingSource { id, state } => (),
        }
    }

    for entity in buffer {
        let id = entity.entity.id;
        let entity = spawn_entity(&mut commands, &assets, entity);
        map.insert(id, entity);
    }
}

fn spawn_entity(
    commands: &mut Commands,
    assets: &AssetServer,
    entity: DelayedEntity,
) -> bevy::ecs::entity::Entity {
    match &entity.entity.body {
        EntityBody::Terrain(terrain) => {
            let id = commands
                .spawn(LoadTerrain {
                    cell: terrain.cell,
                    mesh: terrain.clone(),
                })
                .insert(TransformBundle {
                    local: entity.entity.transform,
                    global: Default::default(),
                })
                .insert(VisibilityBundle::new())
                .insert(entity.entity)
                .id();

            id
        }
        EntityBody::Object(object) => {
            let id = commands
                .spawn(
                    ObjectBundle::new(object.id)
                        .translation(entity.entity.transform.translation)
                        .rotation(entity.entity.transform.rotation),
                )
                .insert(entity.entity)
                .id();

            id
        }
        EntityBody::Actor(act) => {
            let mut actor = ActorBundle::default();
            actor.transform.transform.translation = entity.entity.transform.translation;
            actor.transform.transform.rotation = entity.entity.transform.rotation;
            actor.combat.health = act.health;

            actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

            let mut cmds = commands.spawn(actor);
            cmds.insert(entity.entity);
            Human::default().spawn(assets, &mut cmds);

            if entity.host {
                cmds.insert(HostPlayer).insert(StreamingSource::new());
            }

            cmds.id()
        }
        EntityBody::Item(item) => {
            let id = commands
                .spawn(LoadItem::new(item.id))
                .insert(TransformBundle {
                    local: entity.entity.transform,
                    global: Default::default(),
                })
                .insert(VisibilityBundle::new())
                .insert(entity.entity)
                .id();

            id
        }
    }
}

struct DelayedEntity {
    entity: Entity,
    host: bool,
}

impl From<Entity> for DelayedEntity {
    fn from(value: Entity) -> Self {
        Self {
            entity: value,
            host: false,
        }
    }
}
