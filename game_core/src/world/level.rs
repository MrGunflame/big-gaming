use bevy::prelude::{Query, Res, Transform};
use game_common::components::transform::PreviousTransform;
use game_common::world::{CellId, Level, StreamingSource};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelPlugin;

impl bevy::app::Plugin for LevelPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Level::new()).add_system(load_streaming);
    }
}

impl LevelPlugin {}

fn load_streaming(
    level: Res<Level>,
    sources: Query<(&Transform, &PreviousTransform, &StreamingSource)>,
) {
    for (transform, prev, source) in &sources {
        let new_id = CellId::from(transform.translation);
        let prev_id = CellId::from(prev.translation);

        // No need to reload cells when position didn't change.
        if new_id == prev_id {
            continue;
        }

        tracing::info!("Loading cell {:?}", new_id.as_parts());

        level.load(new_id);
    }
}
