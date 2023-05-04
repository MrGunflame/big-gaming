use bevy_ecs::prelude::Entity;
use bevy_ecs::query::Added;
use bevy_ecs::system::{Query, ResMut};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use crate::{Window, Windows};

pub(crate) fn create_windows(
    event_loop: &EventLoopWindowTarget<()>,
    mut windows: ResMut<Windows>,
    mut created_windows: Query<(Entity, &mut Window), Added<Window>>,
) {
    for (entity, window) in &mut created_windows {
        let window = WindowBuilder::new()
            .with_title(&window.title)
            .build(&event_loop)
            .unwrap();

        windows.windows.insert(window.id(), (window, entity));
    }
}
