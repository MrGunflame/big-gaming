use tokio::runtime::Runtime;
use wgpu::SurfaceError;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::State;

pub async fn run() {
    let event_loop = EventLoop::new();
    let mut window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { window_id, event } => {
            if window_id == state.window().id() {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size);
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor: _,
                        new_inner_size,
                    } => {
                        state.resize(*new_inner_size);
                    }
                    WindowEvent::MouseInput {
                        device_id,
                        state: s,
                        button,
                        modifiers,
                    } => {
                        state.input(&WindowEvent::MouseInput {
                            device_id,
                            state: s,
                            button,
                            modifiers,
                        });
                    }
                    WindowEvent::CursorMoved {
                        device_id,
                        position,
                        modifiers,
                    } => {
                        state.input(&WindowEvent::CursorMoved {
                            device_id,
                            position,
                            modifiers,
                        });
                    }
                    _ => (),
                }
            }
        }
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            state.update();
            match state.render() {
                Ok(_) => (),
                Err(SurfaceError::Lost) => state.resize(state.size),
                Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(err) => println!("{:?}", err),
            }
        }
        Event::MainEventsCleared => {
            state.window.request_redraw();
        }
        _ => (),
    });
}
