use game_common::components::object::ObjectId;
use game_common::components::transform::Transform;
use game_render::Renderer;
use game_scene::{SceneId, Scenes};
use game_tracing::trace_span;

#[derive(Clone, Debug)]
pub struct SpawnObject {
    pub id: ObjectId,
    pub transform: Transform,
}

pub fn spawn_object(scenes: &mut Scenes, renderer: &mut Renderer, object: SpawnObject) -> SceneId {
    let _span = trace_span!("spawn_object").entered();

    let id = scenes.load("../../sponza.glb");
    id
}
