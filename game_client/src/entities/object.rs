use game_common::components::object::ObjectId;
use game_common::components::transform::Transform;
use game_core::modules::Modules;
use game_render::Renderer;
use game_scene::{SceneId, Scenes};
use game_tracing::trace_span;

#[derive(Clone, Debug)]
pub struct SpawnObject {
    pub id: ObjectId,
    pub transform: Transform,
}

impl SpawnObject {
    pub fn spawn(self, scenes: &mut Scenes, modules: &Modules) -> Option<SceneId> {
        let _span = trace_span!("spawn_object").entered();

        let module = modules.get(self.id.0.module)?;
        let record = module.records.get(self.id.0.record)?;
        let body = record.body.as_object()?;

        Some(scenes.load(&body.uri))
    }
}
