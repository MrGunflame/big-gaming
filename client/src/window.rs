use bevy::prelude::{App, Resource};
use bevy::window::Windows;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};

pub fn bind(mut app: App) {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    app.insert_resource(Windows::default());
    app.insert_resource(PrimaryWindow { window });

    let runner = run_world(app);

    event_loop.run(runner);
}

pub fn run_world<'a>(
    mut app: App,
) -> impl FnMut(Event<()>, &EventLoopWindowTarget<()>, &mut ControlFlow) + 'a {
    event_handler(app)
}

fn event_handler<'a>(
    mut app: App,
) -> impl FnMut(Event<()>, &EventLoopWindowTarget<()>, &mut ControlFlow) + 'a {
    move |event, event_loop, control_flow| {
        let world = &mut app.world;

        match event {
            Event::NewEvents(start) => {}
            Event::WindowEvent { window_id, event } => {
                let window = world.resource_mut::<PrimaryWindow>();

                match event {
                    WindowEvent::Resized(_) => {}
                    WindowEvent::CloseRequested => {
                        std::process::exit(0);
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        world.send_event(input);
                    }
                    WindowEvent::CursorMoved {
                        device_id,
                        position,
                        modifiers,
                    } => {}
                    WindowEvent::CursorEntered { device_id } => {}
                    WindowEvent::CursorLeft { device_id } => {}
                    WindowEvent::MouseInput {
                        device_id,
                        state,
                        button,
                        modifiers,
                    } => {}
                    WindowEvent::MouseWheel {
                        device_id,
                        delta,
                        phase,
                        modifiers,
                    } => {}
                    WindowEvent::Touch(_) => {}
                    WindowEvent::ReceivedCharacter(_) => {}
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {}
                    WindowEvent::Focused(_) => {}
                    WindowEvent::DroppedFile(_) => {}
                    WindowEvent::HoveredFile(_) => {}
                    WindowEvent::HoveredFileCancelled => {}
                    WindowEvent::Moved(position) => {}
                    _ => (),
                }
            }
            Event::DeviceEvent { device_id, event } => {
                //
                match event {
                    DeviceEvent::MouseMotion { delta } => {}
                    _ => (),
                }
            }
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => {
                app.update();
            }
            Event::RedrawEventsCleared => {}
            _ => (),
        }
    }
}

#[derive(Debug, Resource)]
pub struct PrimaryWindow {
    window: Window,
}
