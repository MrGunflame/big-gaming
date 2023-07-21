use bevy_app::Plugin;
use game_common::events::{Event, EventQueue};
use game_common::world::world::WorldState;

use crate::scripts::Scripts;
use crate::ScriptServer;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(Scripts::new());
        app.insert_resource(EventQueue::new());
    }
}

pub fn flush_event_queue(
    queue: &mut EventQueue,
    world: &mut WorldState,
    server: &ScriptServer,
    scripts: &Scripts,
    physics_pipeline: &game_physics::Pipeline,
) {
    tracing::debug!("executing {} events", queue.len());

    while let Some(event) = queue.pop() {
        let entity = match event {
            Event::Action(event) => Some(event.entity),
            Event::Collision(event) => Some(event.entity),
            Event::Equip(event) => Some(event.entity),
            Event::Unequip(event) => Some(event.entity),
            Event::CellLoad(_) => None,
            Event::CellUnload(_) => None,
        };

        // FIXME: Optimally we wouldn't event push the event if it is not handled.
        let Some(scripts) = scripts.get(entity, event.kind()) else {
            continue;
        };

        for handle in scripts {
            let Some(view) = world.front_mut() else {
                return;
            };

            let mut instance = server.get(&handle, view, physics_pipeline).unwrap();

            if let Err(err) = instance.run(&event) {
                tracing::error!("failed to execute event on script: {}", err);
            }
        }
    }
}
