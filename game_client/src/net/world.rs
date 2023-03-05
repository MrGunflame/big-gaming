use std::time::{Duration, Instant};

use bevy::prelude::{Commands, Query, Res, ResMut, Transform};
use bevy::time::Time;
use bevy_rapier3d::prelude::Collider;
use game_common::bundles::{ActorBundle, ObjectBundle};
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

    // if id < 60 {
    //     return;
    // }

    // id -= 60;

    // Create a delta from the previous snapshot to the current one.
    let prev = world.get(id - 1);
    let next = world.get(id).unwrap();

    dbg!(&prev);
    dbg!(&next);

    let delta = WorldViewRef::delta(prev, next);

    if !delta.is_empty() {
        for change in delta {
            dbg!(&change);
            queue.push(change);
        }
    }

    // if prev.is_some() {
    //     world.remove(id - 1);
    // }
    snapshots.push();
    world.insert(snapshots.newest().unwrap());
}

pub fn flush_delta_queue(
    mut commands: Commands,
    mut queue: ResMut<DeltaQueue>,
    mut entities: Query<&mut Transform>,
    map: ResMut<EntityMap>,
) {
    while let Some(change) = queue.peek() {
        dbg!(&change);
        match change {
            EntityChange::Create { id, data } => {
                let entity = spawn_entity(&mut commands, data.clone());
                map.insert(*id, entity);
            }
            EntityChange::Destroy { id } => {
                let entity = map.get(*id).unwrap();

                commands.entity(entity).despawn();
            }
            EntityChange::Translate { id, translation } => {
                let entity = map.get(*id).unwrap();

                if let Ok(mut transform) = entities.get_mut(entity) {
                    transform.translation = *translation;
                } else {
                    tracing::warn!("unknown entity");
                }
            }
            EntityChange::Rotate { id, rotation } => {
                let entity = map.get(*id).unwrap();

                if let Ok(mut transform) = entities.get_mut(entity) {
                    transform.rotation = *rotation;
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

fn spawn_entity(commands: &mut Commands, entity: Entity) -> bevy::ecs::entity::Entity {
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
        EntityData::Actor {} => {
            let mut actor = ActorBundle::default();
            actor.transform.transform.translation = entity.transform.translation;
            actor.transform.transform.rotation = entity.transform.rotation;
            actor.physics.collider = Collider::cuboid(1.0, 1.0, 1.0);

            let id = commands.spawn(actor).insert(entity).id();

            id
        }
    }
}
