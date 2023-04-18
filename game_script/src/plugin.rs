use std::collections::VecDeque;

use bevy_app::Plugin;
use bevy_ecs::system::{Res, ResMut, Resource};
use game_common::entity::EntityId;
use game_common::world::world::WorldState;

use crate::events::Event;
use crate::scripts::Scripts;
use crate::ScriptServer;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(Scripts::new());
        app.insert_resource(ScriptQueue::new());

        app.add_system(execute_scripts);
    }
}

#[derive(Clone, Debug, Resource)]
pub struct ScriptQueue {
    events: VecDeque<EntityEvent>,
}

impl ScriptQueue {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn push(&mut self, event: EntityEvent) {
        self.events.push_back(event);
    }

    fn pop(&mut self) -> Option<EntityEvent> {
        self.events.pop_front()
    }
}

/// Run an event on an entity.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntityEvent {
    pub entity: EntityId,
    pub event: Event,
}

fn execute_scripts(
    mut queue: ResMut<ScriptQueue>,
    mut world: ResMut<WorldState>,
    mut server: Res<ScriptServer>,
    mut scripts: Res<Scripts>,
) {
    while let Some(event) = queue.pop() {
        let Some(handles) = scripts.get(event.entity, event.event) else {
            continue;
        };

        for handle in handles {
            let Some(mut view) = world.front_mut() else {
                return;
            };

            let mut instance = server.get(handle, view).unwrap();

            instance.run(event.event);
        }
    }
}
