use game_common::components::race::RaceId;
use game_common::components::transform::Transform;
use game_core::hierarchy::Entity;
use game_core::modules::Modules;
use game_scene::Scenes;
use game_tracing::trace_span;

use crate::utils::extract_actor_rotation;

#[derive(Copy, Clone, Debug)]
pub struct SpawnActor {
    pub race: RaceId,
    pub entity: Entity,
    // Transform only to extract the correct rotation.
    pub transform: Transform,
}

impl SpawnActor {
    pub fn spawn(mut self, scenes: &mut Scenes, modules: &Modules) {
        let _span = trace_span!("spawn_actor").entered();

        self.transform.rotation = extract_actor_rotation(self.transform.rotation);

        let Some(race) = (|| {
            let module = modules.get(self.race.0.module)?;
            let record = module.records.get(self.race.0.record)?;
            record.body.as_race()
        })() else {
            tracing::error!("failed to load actor");
            return;
        };

        scenes.load(self.entity, "pistol.glb");
    }
}
