use bevy::prelude::{Plugin, Res, ResMut};
use game_common::components::interaction::InteractionQueue;
use game_common::events::{ActionEvent, EntityEvent, Event, EventQueue};
use game_net::snapshot::Command;

use crate::net::ServerConnection;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionsPlugin;

impl Plugin for InteractionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // Need full rework
        app.insert_resource(InteractionQueue::new());
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
