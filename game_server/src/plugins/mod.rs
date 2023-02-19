use bevy::prelude::{Entity, Plugin, Query, Res, Transform};
use game_net::proto::Frame;

use crate::snapshot::CommandQueue;

pub struct ServerPlugins {}

impl Plugin for ServerPlugins {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(flush_command_queue);
    }
}

fn flush_command_queue(queue: Res<CommandQueue>, mut entities: Query<(Entity, &Transform)>) {
    while let Some(frame) = queue.pop() {
        match frame {
            Frame::EntityCreate(frame) => {}
            Frame::EntityDestroy(x) => {}
            Frame::EntityTranslate(_) => (),
            Frame::EntityRotate(_) => (),
            Frame::SpawnHost(_) => (),
        }
    }
}
