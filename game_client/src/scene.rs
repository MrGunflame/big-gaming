use bevy::prelude::{EventReader, EventWriter, Res};
use game_common::scene::{Scene, SceneTransition};
use game_net::snapshot::CommandQueue;

use crate::net::ServerConnection;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenePlugin;

impl bevy::prelude::Plugin for ScenePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<SceneTransition>()
            .add_startup_system(startup_transition)
            // .add_system(despawn_scene)
            .add_system(server_connect);
    }
}

// pub fn despawn_scene(
//     mut commands: Commands,
//     mut events: EventReader<SceneTransition>,
//     mut entities: Query<(Entity, &Scene)>,
// ) {
//     for event in events.iter() {
//         for (entity, scene) in &mut entities {
//             if matches!(&event.to, scene) {
//                 continue;
//             }

//             commands.entity(entity).despawn_recursive();
//         }
//     }
// }

fn startup_transition(mut writer: EventWriter<SceneTransition>) {
    writer.send(SceneTransition {
        // Stub
        from: Scene::Loading,
        to: Scene::MainMenu,
    });
}

fn server_connect(
    conn: Res<ServerConnection>,
    queue: Res<CommandQueue>,
    mut events: EventReader<SceneTransition>,
) {
    for event in events.iter() {
        let addr = match &event.to {
            Scene::ServerConnect { addr } => addr,
            _ => continue,
        };

        conn.connect(queue.clone(), addr);
    }
}
