use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, Query, Res, ResMut, Transform, Vec3,
};
use game_common::actors::human::Human;
use game_common::bundles::{ActorBundle, ObjectBundle};
use game_common::components::actor::ActorProperties;
use game_common::components::combat::Health;
use game_common::components::player::HostPlayer;
use game_common::entity::{Entity, EntityData, EntityMap};
use game_common::world::snapshot::EntityChange;
use game_common::world::source::StreamingSource;
use game_common::world::world::{WorldState, WorldViewRef};
use game_net::backlog::Backlog;
use game_net::snapshot::DeltaQueue;

pub fn apply_world_delta(mut world: ResMut<WorldState>, mut queue: ResMut<DeltaQueue>) {
    let (Some(curr), Some(next)) = (world.at(0), world.at(1)) else {
        return;
    };

    let distance = next.creation() - curr.creation();
    dbg!(distance);

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
        &mut Transform,
        Option<&mut Health>,
        Option<&mut ActorProperties>,
    )>,
    map: Res<EntityMap>,
    assets: Res<AssetServer>,
    mut backlog: ResMut<Backlog>,
) {
    // Since events are received in batches, and commands are not applied until
    // the system is done, we buffer all created entities so we can modify them
    // in place within the same batch before they are spawned into the world.
    let mut buffer: Vec<DelayedEntity> = vec![];

    while let Some(change) = queue.pop() {
        match change {
            EntityChange::Create { id, data } => {
                tracing::info!("spawning entity {:?}", id);

                buffer.push(data.into());
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

                if let Ok((mut transform, _, _)) = entities.get_mut(entity) {
                    transform.translation = translation;
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

                if let Ok((mut transform, _, props)) = entities.get_mut(entity) {
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
                        if let EntityData::Actor { race:_, health:h } = &mut entity.entity.data{
                            *h = health;
                        }
                    } else {
                        backlog.push(id, EntityChange::Health { id, health });
                    }

                    continue;
                };

                let (_, h, _) = entities.get_mut(entity).unwrap();
                if let Some(mut h) = h {
                    *h = health;
                } else {
                    tracing::warn!("tried to apply health to a non-actor entity");
                }
            }
            EntityChange::CreateTerrain { cell, height } => {}
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
    match entity.entity.data {
        EntityData::Object { id } => {
            let id = commands
                .spawn(
                    ObjectBundle::new(id)
                        .translation(entity.entity.transform.translation)
                        .rotation(entity.entity.transform.rotation),
                )
                .insert(entity.entity)
                .id();

            id
        }
        EntityData::Actor { race: _, health } => {
            let mut actor = ActorBundle::default();
            actor.transform.transform.translation = entity.entity.transform.translation;
            actor.transform.transform.rotation = entity.entity.transform.rotation;
            actor.combat.health = health;

            actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

            let mut cmds = commands.spawn(actor);
            cmds.insert(entity.entity);
            Human::default().spawn(assets, &mut cmds);

            if entity.host {
                cmds.insert(HostPlayer).insert(StreamingSource::new());
            }

            cmds.id()
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
