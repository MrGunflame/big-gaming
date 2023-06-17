use bevy_ecs::prelude::{Entity, EventReader};
use bevy_ecs::system::{Res, Resource};
use bevy_ecs::world::World;
use game_window::events::WindowDestroyed;
use game_window::Window;

#[derive(Copy, Clone, Debug, Resource)]
pub struct PrimaryWindow(pub Entity);

pub fn spawn_primary_window(world: &mut World) {
    let id = world
        .spawn(Window {
            title: "Game client".to_owned(),
        })
        .id();

    world.insert_resource(PrimaryWindow(id));
}

pub fn destroy_primary_window(
    primary_window: Res<PrimaryWindow>,
    mut events: EventReader<WindowDestroyed>,
) {
    for event in events.iter() {
        if primary_window.0 != event.window {
            continue;
        }

        tracing::info!("Primary window destroyed, exiting");
        std::process::exit(0);
    }
}
