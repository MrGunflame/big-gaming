use bevy_ecs::prelude::{Entity, EventWriter};
use bevy_ecs::query::Added;
use bevy_ecs::system::{Commands, Query, ResMut};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use crate::events::WindowCreated;
use crate::{Window, WindowState, Windows};

pub(crate) fn create_windows(
    mut commands: Commands,
    event_loop: &EventLoopWindowTarget<()>,
    mut windows: ResMut<Windows>,
    mut created_windows: Query<(Entity, &mut Window), Added<Window>>,
    mut writer: EventWriter<WindowCreated>,
) {
    for (entity, window) in &mut created_windows {
        let window = WindowBuilder::new()
            .with_title(&window.title)
            .build(&event_loop)
            .unwrap();

        windows.windows.insert(window.id(), entity);
        commands.entity(entity).insert(WindowState(window));

        writer.send(WindowCreated { window: entity });
    }
}
