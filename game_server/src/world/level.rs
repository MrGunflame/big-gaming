use bevy::prelude::{Query, Transform};
use game_common::components::transform::PreviousTransform;
use game_common::world::source::StreamingSource;

pub fn update_streaming_sources(
    // mut sources: ResMut<StreamingSource>,
    mut entities: Query<(&Transform, &PreviousTransform, &mut StreamingSource)>,
) {
    // sources.clear();

    // for (transform, prev, mut source) in &mut entities {
    //     let new_id = CellId::from(transform.translation);
    //     let prev_id = CellId::from(prev.translation);

    //     if source.state.is_active() && new_id == prev_id {
    //         continue;
    //     }

    //     let mut load = Vec::with_capacity(32);
    //     let mut unload = Vec::with_capacity(32);

    //     // match source.state {
    //     //     StreamingSource::Create => {
    //     //         load.push(new_id);
    //     //     }
    //     // }
    // }
}
