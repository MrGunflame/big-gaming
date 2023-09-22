use game_common::components::transform::Transform;
use game_scene::{Scene, SceneId, Scenes};
use game_tracing::trace_span;

use crate::utils::extract_actor_rotation;

pub fn spawn_actor(scenes: &mut Scenes) -> SceneId {
    let _span = trace_span!("spawn_actor").entered();
    let scene = Scene {
        transform: Transform::default(),
        nodes: vec![],
    };
    scenes.insert(scene)
}
