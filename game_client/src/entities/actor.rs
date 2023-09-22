use game_scene::Scenes;
use game_tracing::trace_span;

use crate::utils::extract_actor_rotation;

pub fn spawn_actor(scenes: &mut Scenes) {
    let _span = trace_span!("spawn_actor").entered();
}
