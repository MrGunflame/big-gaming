mod systems;

use std::collections::HashMap;
use std::time::Instant;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::query::Added;
use bevy_ecs::system::{Query, ResMut, Resource, SystemState};
use bevy_ecs::world::FromWorld;
use systems::create_windows;
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::WindowId;

#[derive(Clone, Debug)]
pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        let event_loop = EventLoop::new();

        app.insert_resource(Windows::default());

        app.insert_non_send_resource(event_loop);
        app.set_runner(main_loop);
    }
}

#[derive(Clone, Debug, Component)]
pub struct Window {
    pub title: String,
}

pub fn main_loop(mut app: App) {
    let event_loop: EventLoop<()> = app.world.remove_non_send_resource().unwrap();

    let mut last_update = Instant::now();

    let mut created_windows_state: SystemState<(
        ResMut<Windows>,
        Query<(Entity, &mut Window), Added<Window>>,
    )> = SystemState::from_world(&mut app.world);

    event_loop.run(move |event, event_loop, control_flow| {
        match event {
            Event::NewEvents(start) => {}
            Event::WindowEvent { window_id, event } => {}
            Event::DeviceEvent { device_id, event } => {}
            Event::UserEvent(()) => (),
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => {
                let should_update = true;

                if should_update {
                    last_update = Instant::now();
                    app.update();
                }
            }
            Event::RedrawEventsCleared => {}
            Event::RedrawRequested(_) => (),
            Event::LoopDestroyed => (),
        }

        let (windows, created_windows) = created_windows_state.get_mut(&mut app.world);
        create_windows(event_loop, windows, created_windows);
        created_windows_state.apply(&mut app.world);
    });
}

#[derive(Debug, Default, Resource)]
struct Windows {
    windows: HashMap<WindowId, (winit::window::Window, Entity)>,
}
