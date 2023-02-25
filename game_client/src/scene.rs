use bevy::prelude::{Commands, DespawnRecursiveExt, Entity, EventReader, EventWriter, Query};
use game_common::scene::{Scene, SceneTransition};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenePlugin;

impl bevy::prelude::Plugin for ScenePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<SceneTransition>()
            .add_startup_system(startup_transition)
            .add_system(despawn_scene);
    }
}

pub fn despawn_scene(
    mut commands: Commands,
    mut events: EventReader<SceneTransition>,
    mut entities: Query<(Entity, &Scene)>,
) {
    for event in events.iter() {
        for (entity, scene) in &mut entities {
            if event.to == *scene {
                continue;
            }

            commands.entity(entity).despawn_recursive();
        }
    }
}

fn startup_transition(mut writer: EventWriter<SceneTransition>) {
    writer.send(SceneTransition {
        // Stub
        from: Scene::Loading,
        to: Scene::MainMenu,
    });
}
