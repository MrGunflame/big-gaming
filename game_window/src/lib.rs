pub mod events;

mod systems;

use std::collections::HashMap;
use std::time::Instant;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Component, Entity, EventWriter};
use bevy_ecs::query::Added;
use bevy_ecs::system::{Commands, Query, ResMut, Resource, SystemState};
use bevy_ecs::world::FromWorld;
use events::{WindowCreated, WindowResized};
use systems::create_windows;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowId;

#[derive(Clone, Debug)]
pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        let event_loop = EventLoop::new();

        app.insert_resource(Windows::default());

        app.add_event::<WindowCreated>();
        app.add_event::<WindowResized>();

        app.insert_non_send_resource(event_loop);
        app.set_runner(main_loop);
    }
}

#[derive(Clone, Debug, Component)]
pub struct Window {
    pub title: String,
}

#[derive(Component)]
pub struct WindowState(pub winit::window::Window);

struct State {
    /// Is the application currently active?
    active: bool,
    /// The timestamp of the last call to `app.update()`.
    last_update: Instant,
}

pub fn main_loop(mut app: App) {
    let event_loop: EventLoop<()> = app.world.remove_non_send_resource().unwrap();

    let mut state = State {
        active: true,
        last_update: Instant::now(),
    };

    let mut created_windows_state: SystemState<(
        Commands,
        ResMut<Windows>,
        Query<(Entity, &mut Window), Added<Window>>,
        EventWriter<WindowCreated>,
    )> = SystemState::from_world(&mut app.world);

    event_loop.run(move |event, event_loop, control_flow| {
        match event {
            Event::NewEvents(start) => {}
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::Resized(size) => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(WindowResized {
                        window,
                        width: size.width,
                        height: size.height,
                    });
                }
                WindowEvent::Moved(_) => {}
                WindowEvent::CloseRequested => {}
                WindowEvent::Destroyed => {}
                WindowEvent::DroppedFile(_) => {}
                WindowEvent::HoveredFile(_) => {}
                WindowEvent::HoveredFileCancelled => {}
                WindowEvent::ReceivedCharacter(_) => {}
                WindowEvent::Focused(_) => {}
                WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                } => {}
                WindowEvent::ModifiersChanged(_) => {}
                WindowEvent::Ime(_) => {}
                WindowEvent::CursorMoved {
                    device_id,
                    position,
                    modifiers,
                } => {}
                WindowEvent::CursorEntered { device_id } => {}
                WindowEvent::CursorLeft { device_id } => {}
                WindowEvent::MouseWheel {
                    device_id,
                    delta,
                    phase,
                    modifiers,
                } => {}
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    modifiers,
                } => {}
                WindowEvent::TouchpadMagnify {
                    device_id,
                    delta,
                    phase,
                } => {}
                WindowEvent::SmartMagnify { device_id } => {}
                WindowEvent::TouchpadRotate {
                    device_id,
                    delta,
                    phase,
                } => {}
                WindowEvent::TouchpadPressure {
                    device_id,
                    pressure,
                    stage,
                } => {}
                WindowEvent::AxisMotion {
                    device_id,
                    axis,
                    value,
                } => {}
                WindowEvent::Touch(_) => {}
                WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                } => {}
                WindowEvent::ThemeChanged(_) => {}
                WindowEvent::Occluded(_) => {}
            },
            Event::DeviceEvent { device_id, event } => {}
            Event::UserEvent(()) => (),
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => {
                let should_update = true;

                if should_update {
                    state.last_update = Instant::now();
                    app.update();
                }

                let mut query = app.world.query::<&WindowState>();
                for (window, entity) in app.world.resource::<Windows>().windows.iter() {
                    let window = query.get(&app.world, *entity).unwrap();
                    window.0.request_redraw();
                }
            }
            Event::RedrawEventsCleared => {}
            Event::RedrawRequested(window_id) => {}
            Event::LoopDestroyed => (),
        }

        let (commands, windows, created_windows, writer) =
            created_windows_state.get_mut(&mut app.world);
        create_windows(commands, event_loop, windows, created_windows, writer);
        created_windows_state.apply(&mut app.world);
    });
}

#[derive(Debug, Default, Resource)]
struct Windows {
    windows: HashMap<WindowId, Entity>,
}
