use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_core::modules::Modules;
use game_scene::{Scene, SceneId, Scenes};
use game_tracing::trace_span;

use crate::utils::extract_actor_rotation;

#[derive(Copy, Clone, Debug)]
pub struct SpawnActor {
    pub race: RaceId,
    pub transform: Transform,
}

impl SpawnActor {
    pub fn spawn(mut self, scenes: &mut Scenes, modules: &Modules) -> Option<SceneId> {
        let _span = trace_span!("spawn_actor").entered();

        self.transform.rotation = extract_actor_rotation(self.transform.rotation);

        let module = modules.get(self.race.0.module)?;
        let record = module.records.get(self.race.0.record)?;
        let race = record.body.as_race()?;

        let scene = Scene {
            transform: Transform::default(),
            nodes: vec![],
        };
        Some(scenes.insert(scene))
    }
}
