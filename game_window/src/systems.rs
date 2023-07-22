use std::sync::Arc;

use bevy_ecs::prelude::{Entity, EventReader, EventWriter};
use bevy_ecs::query::Added;
use bevy_ecs::system::{Commands, Query, ResMut};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use crate::events::{WindowCloseRequested, WindowCreated};
use crate::{Backend, Window, WindowState, Windows};

pub(crate) fn create_windows(
    mut commands: Commands,
    event_loop: &EventLoopWindowTarget<()>,
    mut windows: ResMut<Windows>,
    mut created_windows: Query<(Entity, &mut Window), Added<Window>>,
    mut writer: EventWriter<WindowCreated>,
    backend: Backend,
) {
    for (entity, window) in &mut created_windows {
        let window = WindowBuilder::new()
            .with_title(&window.title)
            .build(&event_loop)
            .unwrap();

        windows.windows.insert(window.id(), entity);
        commands.entity(entity).insert(WindowState {
            inner: Arc::new(window),
            backend,
        });

        writer.send(WindowCreated { window: entity });
    }
}

pub(crate) fn close_requested_windows(
    mut commands: Commands,
    mut events: EventReader<WindowCloseRequested>,
) {
    for event in events.iter() {
        commands.entity(event.window).despawn();
    }
}
