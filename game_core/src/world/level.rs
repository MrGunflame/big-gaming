use bevy::prelude::{AssetServer, Commands, Entity, Query, Res, ResMut, Transform};
use bevy::scene::Scene;
use game_common::archive::GameArchive;
use game_common::components::object::LoadObject;
use game_common::components::transform::PreviousTransform;
use game_common::world::entity::{BuildEntity, EntityQueue};
use game_common::world::source::{StreamingSource, StreamingSources, StreamingState};
use game_common::world::world::WorldState;
use game_common::world::{CellId, Level};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelPlugin;

impl bevy::app::Plugin for LevelPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // app.insert_resource(Level::new())
        // .insert_resource(StreamingSources::new())
        app.insert_resource(EntityQueue::new())
            // .add_system(update_streaming_sources)
            // .add_system(process_queue)
            .add_system(flush_entity_queue)
            .add_system(load_objects);
    }
}

impl LevelPlugin {}

// fn update_streaming_sources(
//     mut sources: ResMut<StreamingSources>,
//     mut query: Query<(&Transform, &PreviousTransform, &mut StreamingSource)>,
// ) {
//     sources.clear();

//     for (transform, prev, mut source) in &mut query {
//         let new_id = CellId::from(transform.translation);
//         let prev_id = CellId::from(prev.translation);

//         // No need to reload cells when position didn't change.
//         if source.state.is_active() && new_id == prev_id {
//             continue;
//         }

//         let mut load = Vec::with_capacity(32);
//         let mut unload = Vec::with_capacity(32);

//         match source.state {
//             StreamingState::Create => {
//                 load.push(new_id);

//                 source.state = StreamingState::Active;
//             }
//             StreamingState::Active => {
//                 load.push(new_id);
//                 unload.push(prev_id);
//             }
//             StreamingState::Destroy => {
//                 unload.push(new_id);

//                 source.state = StreamingState::Destroyed;
//             }
//             StreamingState::Destroyed => continue,
//         }

//         for id in load {
//             sources.load(id);
//         }

//         for id in unload {
//             sources.unload(id);
//         }
//     }
// }

// fn process_queue(
//     sources: Res<StreamingSources>,
//     mut queue: ResMut<EntityQueue>,
//     level: Res<Level>,
// ) {
//     for id in sources.loaded() {
//         tracing::info!("loading cell {:?}", id);
//         let cell = level.load(id);
//     }

//     for id in sources.unloaded() {
//         tracing::info!("unloading cell {:?}", id);
//     }
// }

fn flush_entity_queue(mut world: ResMut<WorldState>, mut queue: ResMut<EntityQueue>) {
    let Some(mut view) = world.back_mut() else {
        return;
    };

    while let Some(entity) = queue.pop() {
        tracing::info!("building entity {:?}", entity);
        entity.build(&mut view);
    }
}

fn load_objects(
    mut commands: Commands,
    archive: Res<GameArchive>,
    assets: Res<AssetServer>,
    objects: Query<(Entity, &LoadObject)>,
) {
    for (entity, object) in &objects {
        tracing::info!("loading object {:?}", object.id);

        let Some(obj) = archive.objects().get(object.id) else {
            continue;
        };

        commands.entity(entity).remove::<LoadObject>();

        let Some(handle) = &obj.handle else {
            continue;
        };

        commands
            .entity(entity)
            .insert(assets.load::<Scene, _>(handle));
    }
}
