pub mod cursor;
pub mod events;
pub mod windows;

mod backend;

use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc};

use backend::Backend;
use cursor::{Cursor, CursorGrabMode, WindowCompat};
use events::{
    convert_key_code, CursorEntered, CursorLeft, CursorMoved, WindowCloseRequested, WindowCreated,
    WindowDestroyed, WindowResized,
};
use game_input::keyboard::{KeyboardInput, ScanCode};
use game_input::mouse::{MouseButton, MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use game_input::ButtonState;
use game_tracing::trace_span;
use glam::Vec2;
use windows::{UpdateEvent, WindowState, Windows};
use winit::event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::{WindowBuilder, WindowId};

/// The entrypoint for interacting with the OS windowing system.
#[derive(Debug)]
pub struct WindowManager {
    state: WindowManagerState,
    windows: Windows,
    cursor: Arc<Cursor>,
}

impl WindowManager {
    /// Creates a new `WindowManager`.
    ///
    /// Note that this does not automatically create a window. To create a new window when the
    /// application lauches you can call [`Windows::spawn`] before calling [`WindowManager::run`].
    ///
    /// [`Windows::spawn`]: Windows::spawn
    /// [`WindowManager::run`]: WindowManager::run
    pub fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        let (update_tx, update_rx) = mpsc::channel();
        let windows = windows::Windows::new(update_tx.clone());
        let cursor = Arc::new(Cursor::new(update_tx));

        Self {
            state: WindowManagerState {
                event_loop,
                update_rx,
                cursor: cursor.clone(),
            },
            windows,
            cursor,
        }
    }

    /// Returns a reference to the active [`Windows`].
    #[inline]
    pub fn windows(&self) -> &Windows {
        &self.windows
    }

    /// Returns a mutable reference to the active [`Windows`].
    #[inline]
    pub fn windows_mut(&mut self) -> &mut Windows {
        &mut self.windows
    }

    pub fn cursor(&self) -> &Arc<Cursor> {
        &self.cursor
    }

    /// Starts the `WindowManager` using the given [`App`].
    ///
    /// Note that the call to `run` will never return.
    pub fn run<T>(self, app: T) -> !
    where
        T: App,
    {
        main_loop(self.state, self.windows, app);
    }
}

impl Default for WindowManager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct WindowManagerState {
    event_loop: EventLoop<()>,
    update_rx: mpsc::Receiver<UpdateEvent>,
    cursor: Arc<Cursor>,
}

fn main_loop<T>(state: WindowManagerState, mut windows: Windows, mut app: T) -> !
where
    T: App,
{
    let event_loop = state.event_loop;
    let update_rx = state.update_rx;
    let cursor = state.cursor;

    let backend = Backend::from(&event_loop);

    let mut map = WindowMap::default();

    let mut compat = WindowCompat::new(backend);
    let mut is_locked = false;

    // `ControlFlow::Poll` is required to constantly trigger `AboutToWait` events.
    // By default the control flow is set to `Wait` which causes the game to stall
    // whenever there are no OS events.
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run(move |event, event_loop| {
            match event {
                Event::NewEvents(_start) => {}
                Event::WindowEvent { window_id, event } => {
                    let window = *map.windows.get(&window_id).unwrap();

                    match event {
                        WindowEvent::Resized(size) => {
                            let event = events::WindowEvent::WindowResized(WindowResized {
                                window,
                                width: size.width,
                                height: size.height,
                            });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );
                        }
                        WindowEvent::CloseRequested => {
                            let event =
                                events::WindowEvent::WindowCloseRequested(WindowCloseRequested {
                                    window,
                                });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );
                        }
                        WindowEvent::Destroyed => {
                            let window = map.windows.remove(&window_id).unwrap();

                            let event =
                                events::WindowEvent::WindowDestroyed(WindowDestroyed { window });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            let scan_code = ScanCode(event.physical_key.to_scancode().unwrap());
                            let text = event.logical_key.to_text().map(|s| s.into());
                            let key_code = match event.physical_key {
                                PhysicalKey::Code(key) => convert_key_code(key),
                                PhysicalKey::Unidentified(_) => None,
                            };

                            let event = events::WindowEvent::KeyboardInput(KeyboardInput {
                                scan_code,
                                key_code,
                                text,
                                state: match event.state {
                                    ElementState::Pressed => ButtonState::Pressed,
                                    ElementState::Released => ButtonState::Released,
                                },
                            });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );
                        }
                        WindowEvent::CursorMoved {
                            device_id: _,
                            position,
                            ..
                        } => {
                            let event = events::WindowEvent::CursorMoved(CursorMoved {
                                window,
                                position: Vec2::new(position.x as f32, position.y as f32),
                            });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );

                            compat.move_cursor();

                            if !is_locked {
                                let mut cursor_state = cursor.state.write();
                                cursor_state.position =
                                    Vec2::new(position.x as f32, position.y as f32);
                                cursor_state.window = Some(window);
                                compat
                                    .set_position(Vec2::new(position.x as f32, position.y as f32));
                            }
                        }
                        WindowEvent::CursorEntered { device_id: _ } => {
                            let event =
                                events::WindowEvent::CursorEntered(CursorEntered { window });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );
                        }
                        WindowEvent::CursorLeft { device_id: _ } => {
                            let event = events::WindowEvent::CursorLeft(CursorLeft { window });
                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                event,
                            );

                            if !is_locked {
                                let mut cursor_state = cursor.state.write();
                                cursor_state.window = None;
                            }
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            // `winit 0.28.4` does not emit `DeviceEvent::MouseWheel` for wayland
                            // event loops. Whether this is a bug or a "feature" is to be determined.
                            // Until then we have to manually capture MouseWheel events from the window
                            // and ignore `DeviceEvent::MouseWheel` (in case the behavoir changes in the
                            // future).
                            if backend.is_wayland() {
                                let event = match delta {
                                    // Direction is inverted compared to X11.
                                    MouseScrollDelta::LineDelta(x, y) => MouseWheel {
                                        unit: MouseScrollUnit::Line,
                                        x: -x,
                                        y: -y,
                                    },
                                    MouseScrollDelta::PixelDelta(pos) => MouseWheel {
                                        unit: MouseScrollUnit::Pixel,
                                        x: pos.x as f32,
                                        y: pos.y as f32,
                                    },
                                };

                                let event = events::WindowEvent::MouseWheel(event);
                                app.handle_event(
                                    WindowManagerContext {
                                        windows: &mut windows,
                                    },
                                    event,
                                );
                            }
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let event = MouseButtonInput {
                                button: match button {
                                    winit::event::MouseButton::Left => MouseButton::Left,
                                    winit::event::MouseButton::Right => MouseButton::Right,
                                    winit::event::MouseButton::Middle => MouseButton::Middle,
                                    winit::event::MouseButton::Back => MouseButton::Back,
                                    winit::event::MouseButton::Forward => MouseButton::Forward,
                                    winit::event::MouseButton::Other(button) => {
                                        MouseButton::Other(button)
                                    }
                                },
                                state: match state {
                                    ElementState::Pressed => ButtonState::Pressed,
                                    ElementState::Released => ButtonState::Released,
                                },
                            };

                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                events::WindowEvent::MouseButtonInput(event),
                            );
                        }
                        WindowEvent::RedrawRequested => {}
                        _ => (),
                    }
                }
                Event::DeviceEvent { event, .. } => match event {
                    DeviceEvent::MouseMotion { delta: (x, y) } => {
                        let event = MouseMotion {
                            delta: Vec2 {
                                x: x as f32,
                                y: y as f32,
                            },
                        };

                        app.handle_event(
                            WindowManagerContext {
                                windows: &mut windows,
                            },
                            events::WindowEvent::MouseMotion(event),
                        );
                    }
                    DeviceEvent::MouseWheel { delta } => {
                        // See comment at `WindowEvent::MouseWheel` for
                        // why this is necessary.
                        if !backend.is_wayland() {
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

                            app.handle_event(
                                WindowManagerContext {
                                    windows: &mut windows,
                                },
                                events::WindowEvent::MouseWheel(event),
                            );
                        }
                    }
                    _ => (),
                },
                Event::AboutToWait => {
                    trace_span!("AboutToWait");

                    app.update(WindowManagerContext {
                        windows: &mut windows,
                    });
                }
                _ => (),
            }

            let mut queue = VecDeque::new();

            // Run compat events before custom generated events so
            // that custom events can still overwrite compat
            // behavior.
            compat.emulate_cursor_grab_mode_locked(&cursor, &mut queue);

            while let Ok(event) = update_rx.try_recv() {
                queue.push_back(event);
            }

            while let Some(event) = queue.pop_front() {
                match event {
                    UpdateEvent::Create(id) => {
                        let window = windows.get(id).unwrap();

                        let window = WindowBuilder::new()
                            .with_title(window.title.clone())
                            .build(event_loop)
                            .expect("failed to create window");

                        map.windows.insert(window.id(), id);

                        windows.get_mut(id).unwrap().state = Some(WindowState {
                            id,
                            inner: Arc::new(window),
                            backend,
                        });

                        app.handle_event(
                            WindowManagerContext {
                                windows: &mut windows,
                            },
                            events::WindowEvent::WindowCreated(WindowCreated { window: id }),
                        );
                    }
                    UpdateEvent::Destroy(id) => {
                        app.handle_event(
                            WindowManagerContext {
                                windows: &mut windows,
                            },
                            events::WindowEvent::WindowDestroyed(WindowDestroyed { window: id }),
                        );
                    }
                    UpdateEvent::CursorGrab(id, mode) => {
                        let Some(window) = windows.get(id) else {
                            continue;
                        };

                        if let Err(err) = window
                            .state
                            .as_ref()
                            .expect("window not initialized")
                            .set_cursor_grab(mode)
                        {
                            tracing::error!("failed to set cursor grab mode: {}", err);
                        }

                        let mut cursor_state = cursor.state.write();
                        match mode {
                            CursorGrabMode::None => {
                                cursor_state.is_locked = false;
                                is_locked = false;
                                compat.unlock_cursor();
                            }
                            CursorGrabMode::Locked => {
                                cursor_state.is_locked = true;
                                is_locked = true;
                                compat.lock_cursor();
                            }
                        }
                    }
                    UpdateEvent::CursorVisible(id, visible) => {
                        let Some(window) = windows.get(id) else {
                            continue;
                        };

                        window
                            .state
                            .as_ref()
                            .expect("window not initialized")
                            .inner
                            .set_cursor_visible(visible);
                    }
                    UpdateEvent::CursorPosition(id, position) => {
                        let Some(window) = windows.get(id) else {
                            continue;
                        };

                        if let Err(err) = window
                            .state
                            .as_ref()
                            .expect("window not initialized")
                            .set_cursor_position(position)
                        {
                            tracing::error!("failed to set cursor position: {}", err);
                        }

                        let mut cursor_state = cursor.state.write();
                        cursor_state.window = Some(id);
                        cursor_state.position = position;
                    }
                }
            }
        })
        .unwrap();

    std::process::exit(0);
}

#[derive(Clone, Debug, Default)]
struct WindowMap {
    windows: HashMap<WindowId, windows::WindowId>,
}

pub trait App: 'static {
    fn handle_event(&mut self, ctx: WindowManagerContext<'_>, event: events::WindowEvent);

    fn update(&mut self, ctx: WindowManagerContext<'_>);
}

#[derive(Debug)]
#[non_exhaustive]
pub struct WindowManagerContext<'a> {
    pub windows: &'a mut Windows,
}
