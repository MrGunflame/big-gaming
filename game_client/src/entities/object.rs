use game_common::components::object::ObjectId;
use game_core::modules::Modules;
use game_data::components::objects::ObjectRecord;
use game_scene::scene2::Key;
use game_tracing::trace_span;

use crate::scene::SceneState;

#[derive(Clone, Debug)]
pub struct SpawnObject {
    pub id: ObjectId,
    pub key: Key,
}

impl SpawnObject {
    pub fn spawn(self, scenes: &mut SceneState, modules: &Modules) {
        let _span = trace_span!("spawn_object").entered();

        let Some(body) = (|| -> Option<&ObjectRecord> {
            let module = modules.get(self.id.0.module)?;
            let record = module.records.get(self.id.0.record)?;
            record.body.as_object()
        })() else {
            tracing::error!("failed to load object");
            return;
        };

        scenes.spawner.spawn(self.key, &body.uri);
    }
}
