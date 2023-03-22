use bevy::prelude::{
    AssetServer, Commands, DespawnRecursiveExt, Query, Res, ResMut, Transform, Vec3,
};
use game_common::actors::human::Human;
use game_common::bundles::{ActorBundle, ObjectBundle};
use game_common::components::actor::ActorProperties;
use game_common::components::combat::Health;
use game_common::components::player::HostPlayer;
use game_common::entity::{Entity, EntityData, EntityMap};
use game_common::world::source::StreamingSource;
use game_net::snapshot::{DeltaQueue, EntityChange};
use game_net::world::{WorldState, WorldViewRef};

pub fn apply_world_delta(mut world: ResMut<WorldState>, mut queue: ResMut<DeltaQueue>) {
    let Some(view) = world.next() else {
        return;
    };

    let delta = WorldViewRef::delta(view.prev, view.view);

    for change in delta {
        queue.push(change);
    }

    if world.len() > 120 {
        world.pop();
    }
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
) {
    while let Some(change) = queue.peek() {
        match change {
            EntityChange::Create { id, data } => {
                let entity = spawn_entity(&mut commands, &assets, data.clone());
                map.insert(*id, entity);

                tracing::info!("spawning entity {:?}", id);

                // The following commands may reference an entity that was just created.
                // Wait for the next tick before processing them.
                // TODO: This should rather update the entities in place instead of waiting.
                queue.pop().unwrap();
                return;
            }
            EntityChange::Destroy { id } => {
                let Some(entity) = map.get(*id) else {
                    tracing::warn!("attempted to destroy a non-existent entity: {:?}", id);
                    queue.pop().unwrap();
                    continue;
                };

                tracing::info!("despawning entity {:?}", id);

                commands.entity(entity).despawn_recursive();
            }
            EntityChange::Translate {
                id,
                translation,
                cell: _,
            } => {
                let entity = map.get(*id).unwrap();

                if let Ok((mut transform, _, _)) = entities.get_mut(entity) {
                    transform.translation = *translation;
                } else {
                    tracing::warn!("unknown entity");
                }
            }
            EntityChange::Rotate { id, rotation } => {
                let entity = map.get(*id).unwrap();

                if let Ok((mut transform, _, props)) = entities.get_mut(entity) {
                    if let Some(mut props) = props {
                        // Actor
                        props.rotation = *rotation;
                    } else {
                        // Object
                        transform.rotation = *rotation;
                    }
                } else {
                    tracing::warn!("unknown entity");
                }
            }
            EntityChange::CreateHost { id } => {
                let entity = map.get(*id).unwrap();

                commands
                    .entity(entity)
                    .insert(HostPlayer)
                    .insert(StreamingSource::new());
            }
            EntityChange::DestroyHost { id } => {
                let entity = map.get(*id).unwrap();

                commands
                    .entity(entity)
                    .remove::<HostPlayer>()
                    .remove::<StreamingSource>();
            }
            EntityChange::Health { id, health } => {
                let entity = map.get(*id).unwrap();

                let (_, h, _) = entities.get_mut(entity).unwrap();
                if let Some(mut h) = h {
                    *h = *health;
                } else {
                    tracing::warn!("tried to apply health to a non-actor entity");
                }
            }
        }

        queue.pop().unwrap();
    }
}

fn spawn_entity(
    commands: &mut Commands,
    assets: &AssetServer,
    entity: Entity,
) -> bevy::ecs::entity::Entity {
    match entity.data {
        EntityData::Object { id } => {
            let id = commands
                .spawn(
                    ObjectBundle::new(id)
                        .translation(entity.transform.translation)
                        .rotation(entity.transform.rotation),
                )
                .insert(entity)
                .id();

            id
        }
        EntityData::Actor { race: _, health } => {
            let mut actor = ActorBundle::default();
            actor.transform.transform.translation = entity.transform.translation;
            actor.transform.transform.rotation = entity.transform.rotation;
            actor.combat.health = health;

            actor.properties.eyes = Vec3::new(0.0, 1.6, -0.1);

            let mut cmds = commands.spawn(actor);
            cmds.insert(entity);
            Human::default().spawn(assets, &mut cmds);

            cmds.id()
        }
    }
}
