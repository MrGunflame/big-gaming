use game_common::components::object::ObjectId;
use game_core::hierarchy::Entity;
use game_core::modules::Modules;
use game_data::components::objects::ObjectRecord;
use game_scene::Scenes;
use game_tracing::trace_span;

#[derive(Clone, Debug)]
pub struct SpawnObject {
    pub id: ObjectId,
    pub entity: Entity,
}

impl SpawnObject {
    pub fn spawn(self, scenes: &mut Scenes, modules: &Modules) {
        let _span = trace_span!("spawn_object").entered();

        let Some(body) = (|| -> Option<&ObjectRecord> {
            let module = modules.get(self.id.0.module)?;
            let record = module.records.get(self.id.0.record)?;
            record.body.as_object()
        })() else {
            tracing::error!("failed to load object");
            return;
        };

        scenes.load(self.entity, &body.uri);
    }
}
