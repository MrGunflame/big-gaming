use std::time::{Duration, Instant};

use bevy::prelude::{AssetServer, Commands, Query, Res, ResMut, Transform, Vec3};
use bevy::time::Time;
use bevy_rapier3d::prelude::Collider;
use game_common::actors::human::Human;
use game_common::bundles::{ActorBundle, ObjectBundle};
use game_common::components::actor::ActorProperties;
use game_common::components::combat::Health;
use game_common::components::player::HostPlayer;
use game_common::entity::{Entity, EntityData, EntityMap};
use game_common::world::source::StreamingSource;
use game_net::snapshot::{DeltaQueue, EntityChange, Snapshots};
use game_net::world::{WorldState, WorldViewRef};

pub fn apply_world_delta(
    mut commands: Commands,
    mut world: ResMut<WorldState>,
    mut snapshots: ResMut<Snapshots>,
    map: ResMut<EntityMap>,
    mut queue: ResMut<DeltaQueue>,
) {
    // snapshots.push();
    // world.insert(snapshots.newest().unwrap());

    // let Some(id) = snapshots.get(Instant::now()) else {
    //     return;
    // };

    let Some(mut id) = snapshots.newest() else {
        snapshots.push();
        world.insert(snapshots.newest().unwrap());
        return;
    };

    if id.0 < 6 {
        snapshots.push();
        world.insert(snapshots.newest().unwrap());
        return;
    }

    id -= 6;

    // Create a delta from the previous snapshot to the current one.
    let prev = world.get(id - 1);
    let next = world.get(id).unwrap();

    let delta = WorldViewRef::delta(prev, next);

    for change in delta {
        queue.push(change);
    }

    if prev.is_some() {
        world.remove(id - 1);
        snapshots.remove(id - 1);
    }

    snapshots.push();
    world.insert(snapshots.newest().unwrap());
}

pub fn flush_delta_queue(
    mut commands: Commands,
    mut queue: ResMut<DeltaQueue>,
    mut entities: Query<(
        &mut Transform,
        Option<&mut Health>,
        Option<&mut ActorProperties>,
    )>,
    map: ResMut<EntityMap>,
    assets: Res<AssetServer>,
) {
    while let Some(change) = queue.peek() {
        match change {
            EntityChange::Create { id, data } => {
                let entity = spawn_entity(&mut commands, &assets, data.clone());
                map.insert(*id, entity);

                // The following commands may reference an entity that was just created.
                // Wait for the next tick before processing them.
                // TODO: This should rather update the entities in place instead of waiting.
                queue.pop().unwrap();
                return;
            }
            EntityChange::Destroy { id } => {
                let entity = map.get(*id).unwrap();

                commands.entity(entity).despawn();
            }
            EntityChange::Translate { id, translation } => {
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

pub fn advance_snapshots(
    time: Res<Time>,
    mut snapshots: ResMut<Snapshots>,
    mut world: ResMut<WorldState>,
) {
    // snapshots.push();
    // world.insert(snapshots.newest().unwrap());

    // if snapshots.newest().unwrap().0 - snapshots.oldest().unwrap().0 > 120 {
    //     world.remove(snapshots.oldest().unwrap());
    // }
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
