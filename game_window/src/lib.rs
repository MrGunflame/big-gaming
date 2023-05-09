pub mod events;

mod systems;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Component, Entity, EventWriter};
use bevy_ecs::query::Added;
use bevy_ecs::system::{Commands, Query, ResMut, Resource, SystemState};
use bevy_ecs::world::FromWorld;
use events::{
    CursorEntered, CursorLeft, CursorMoved, ReceivedCharacter, WindowCloseRequested, WindowCreated,
    WindowDestroyed, WindowResized,
};
use game_input::keyboard::{KeyboardInput, ScanCode};
use game_input::mouse::{MouseButton, MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use game_input::{ButtonState, InputPlugin};
use glam::{DVec2, Vec2};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use systems::create_windows;
use winit::dpi::{LogicalPosition, PhysicalSize, Position};
use winit::error::ExternalError;
use winit::event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{CursorGrabMode, WindowId};

#[derive(Clone, Debug)]
pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        let event_loop = EventLoop::new();

        // Input plugin so we can send generic device (keyboard/mouse)
        // events.
        app.add_plugin(InputPlugin);

        app.insert_resource(Windows::default());

        app.add_event::<WindowCreated>();
        app.add_event::<WindowResized>();
        app.add_event::<WindowDestroyed>();
        app.add_event::<CursorMoved>();
        app.add_event::<CursorEntered>();
        app.add_event::<CursorLeft>();
        app.add_event::<WindowCloseRequested>();

        app.add_system(systems::close_requested_windows);

        app.insert_non_send_resource(event_loop);
        app.set_runner(main_loop);
    }
}

#[derive(Clone, Debug, Component)]
pub struct Window {
    pub title: String,
}

#[derive(Clone, Debug, Component)]
pub struct WindowState {
    // Note: It is important that the window handle itself sits
    // behind an Arc and is not immediately dropped once the window
    // component is despawned. Rendering surfaces require the handle
    // to be valid until the surface was dropped in the rendering
    // crate.
    inner: Arc<winit::window::Window>,
}

impl WindowState {
    pub fn inner_size(&self) -> PhysicalSize<u32> {
        self.inner.inner_size()
    }

    pub fn set_cursor_position(&self, position: Vec2) -> Result<(), ExternalError> {
        self.inner
            .set_cursor_position(Position::Logical(LogicalPosition {
                x: position.x as f64,
                y: position.y as f64,
            }))
    }

    pub fn set_cursor_visibility(&self, visible: bool) {
        self.inner.set_cursor_visible(visible);
    }

    pub fn set_cursor_grab(&self, mode: CursorGrabMode) -> Result<(), ExternalError> {
        self.inner.set_cursor_grab(mode)
    }
}

unsafe impl HasRawDisplayHandle for WindowState {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.inner.raw_display_handle()
    }
}

unsafe impl HasRawWindowHandle for WindowState {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.inner.raw_window_handle()
    }
}

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
                WindowEvent::CloseRequested => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(WindowCloseRequested { window });
                }
                WindowEvent::Destroyed => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(WindowDestroyed { window });
                    app.world
                        .resource_mut::<Windows>()
                        .windows
                        .remove(&window_id);

                    if app.world.resource::<Windows>().windows.is_empty() {
                        tracing::debug!("last window destroyed, exiting");
                        std::process::exit(0);
                    }
                }
                WindowEvent::DroppedFile(_) => {}
                WindowEvent::HoveredFile(_) => {}
                WindowEvent::HoveredFileCancelled => {}
                WindowEvent::ReceivedCharacter(char) => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(ReceivedCharacter { window, char });
                }
                WindowEvent::Focused(_) => {}
                WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                } => {
                    app.world.send_event(KeyboardInput {
                        scan_code: ScanCode(input.scancode),
                        key_code: input.virtual_keycode,
                        state: match input.state {
                            ElementState::Pressed => ButtonState::Pressed,
                            ElementState::Released => ButtonState::Released,
                        },
                    });
                }
                WindowEvent::ModifiersChanged(_) => {}
                WindowEvent::Ime(_) => {}
                WindowEvent::CursorMoved {
                    device_id: _,
                    position,
                    modifiers: _,
                } => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(CursorMoved {
                        window,
                        position: Vec2::new(position.x as f32, position.y as f32),
                    });
                }
                WindowEvent::CursorEntered { device_id: _ } => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(CursorEntered { window });
                }
                WindowEvent::CursorLeft { device_id: _ } => {
                    let window = app
                        .world
                        .resource::<Windows>()
                        .windows
                        .get(&window_id)
                        .copied()
                        .unwrap();

                    app.world.send_event(CursorLeft { window });
                }
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
                } => {
                    app.world.send_event(MouseButtonInput {
                        button: match button {
                            winit::event::MouseButton::Left => MouseButton::Left,
                            winit::event::MouseButton::Right => MouseButton::Right,
                            winit::event::MouseButton::Middle => MouseButton::Middle,
                            winit::event::MouseButton::Other(button) => MouseButton::Other(button),
                        },
                        state: match state {
                            ElementState::Pressed => ButtonState::Pressed,
                            ElementState::Released => ButtonState::Released,
                        },
                    });
                }
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
            Event::DeviceEvent { device_id, event } => match event {
                DeviceEvent::Added => {}
                DeviceEvent::Removed => {}
                DeviceEvent::MouseMotion { delta: (x, y) } => {
                    app.world.send_event(MouseMotion {
                        delta: Vec2 {
                            x: x as f32,
                            y: y as f32,
                        },
                    });
                }
                DeviceEvent::MouseWheel { delta } => {
                    let event = match delta {
                        MouseScrollDelta::LineDelta(x, y) => MouseWheel {
                            unit: MouseScrollUnit::Line,
                            x,
                            y,
                        },
                        MouseScrollDelta::PixelDelta(pos) => MouseWheel {
                            unit: MouseScrollUnit::Pixel,
                            x: pos.x as f32,
                            y: pos.y as f32,
                        },
                    };

                    app.world.send_event(event);
                }
                DeviceEvent::Motion { axis, value } => {}
                DeviceEvent::Button { button, state } => {}
                DeviceEvent::Key(key) => {}
                DeviceEvent::Text { codepoint } => {}
            },
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
                for entity in app.world.resource::<Windows>().windows.values() {
                    // If the window entity doesn't exist anymore, it was despawned
                    // in this loop and will get removed in the next update.
                    let Ok(window) = query.get(&app.world, *entity) else {
                        continue;
                    };

                    window.inner.request_redraw();
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
