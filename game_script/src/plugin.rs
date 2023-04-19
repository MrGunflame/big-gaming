use std::collections::VecDeque;

use bevy_app::Plugin;
use bevy_ecs::system::{Res, ResMut, Resource};
use game_common::entity::EntityId;
use game_common::events::{EntityEvent, Event, EventQueue};
use game_common::world::world::WorldState;

use crate::scripts::Scripts;
use crate::{Handle, ScriptServer};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(Scripts::new());
        app.insert_resource(EventQueue::new());

        app.add_system(execute_scripts);
    }
}

fn execute_scripts(
    mut queue: ResMut<EventQueue>,
    mut world: ResMut<WorldState>,
    server: Res<ScriptServer>,
    scripts: Res<Scripts>,
) {
    // while let Some(event) = queue.pop() {
    //     let Some(handles) = scripts.get(event.entity, event.event.kind()) else {
    //         continue;
    //     };

    //     for handle in handles {
    //         let Some(mut view) = world.front_mut() else {
    //             return;
    //         };

    //         let mut instance = server.get(handle, view).unwrap();

    //         instance.run(&event.event);
    //     }
    // }
}

pub fn flush_event_queue(
    queue: &mut EventQueue,
    world: &mut WorldState,
    server: &ScriptServer,
    scripts: &Scripts,
) {
    while let Some(event) = queue.pop() {
        for (i, _) in server.scripts.iter().enumerate() {
            let Some(mut view) = world.front_mut() else {
                return;
            };

            let handle = Handle { id: i as u64 };

            let mut instance = server.get(&handle, view).unwrap();

            instance.run(&event.event);
        }
    }
}
