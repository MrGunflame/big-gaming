use bevy::prelude::{Plugin, Stage};
use game_common::components::interaction::InteractionQueue;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionsPlugin;

impl Plugin for InteractionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(InteractionQueue::new())
            .add_stage("Interaction", InteractionStage);
    }
}

struct InteractionStage;

impl Stage for InteractionStage {
    fn run(&mut self, world: &mut bevy::prelude::World) {
        world.resource_scope::<InteractionQueue, ()>(|world, mut queue| {
            queue.run(world);
        });
    }
}
