use bevy::prelude::{Plugin, Res, ResMut};
use game_common::components::actions::ActionQueue;
use game_common::components::interaction::InteractionQueue;
use game_common::events::{EntityEvent, Event, EventQueue};
use game_net::snapshot::Command;

use crate::net::ServerConnection;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionsPlugin;

impl Plugin for InteractionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // Need full rework
        app.insert_resource(InteractionQueue::new());
        //     .add_stage("Interaction", InteractionStage);

        app.insert_resource(ActionQueue::new());

        app.add_system(handle_action_events);
    }
}

// struct InteractionStage;

// impl Stage for InteractionStage {
//     fn run(&mut self, world: &mut bevy::prelude::World) {
//         world.resource_scope::<InteractionQueue, ()>(|world, mut queue| {
//             queue.run(world);
//         });
//     }
// }

fn handle_action_events(
    conn: Res<ServerConnection>,
    mut actions: ResMut<ActionQueue>,
    mut events: ResMut<EventQueue>,
) {
    while let Some(action) = actions.pop() {
        events.push(EntityEvent {
            entity: action.entity,
            event: Event::Action {
                entity: action.entity,
                invoker: action.entity,
            },
        });

        conn.send(Command::EntityAction {
            id: action.entity,
            action: action.id,
        });
    }
}
