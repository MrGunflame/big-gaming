//! UI related systems

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

use std::collections::HashMap;
use std::sync::{mpsc, Arc};

// We need criterion for benches, but it is incorrectly detected
// by `unused_crate_dependencies`.
#[cfg(test)]
use criterion as _;

pub mod events;
pub mod reactive;
pub mod render;
pub mod widgets;

use events::{Events, WindowCommand};
use game_render::RenderState;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use game_window::WindowManager;
use glam::UVec2;
use reactive::{Document, Runtime};
use render::RenderUiState;

pub use game_ui_macros::{component, view};
use wgpu::{Device, Queue};

pub struct UiState {
    render: RenderUiState,
    windows: HashMap<WindowId, Document>,
    events: HashMap<WindowId, Events>,
    pub runtime: Runtime,
    command_rx: mpsc::Receiver<WindowCommand>,
    command_tx: mpsc::Sender<WindowCommand>,
}

impl UiState {
    pub fn new(render_state: &mut RenderState) -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        Self {
            render: RenderUiState::new(&render_state.device, &mut render_state.graph),
            runtime: Runtime::new(),
            windows: HashMap::new(),
            events: HashMap::new(),
            command_rx,
            command_tx,
        }
    }

    pub fn create(&mut self, id: WindowId, size: UVec2) {
        self.render.insert(id, size);
        self.windows.insert(id, Document::new(self.runtime.clone()));
        self.events.insert(id, Events::new());
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Document> {
        self.windows.get_mut(&id)
    }

    pub fn resize(&mut self, id: WindowId, size: UVec2) {
        self.render.resize(id, size);
    }

    pub fn destroy(&mut self, id: WindowId) {
        self.render.remove(id);
        self.windows.remove(&id);
        self.events.remove(&id);
    }

    pub fn send_event(&mut self, cursor: &Arc<Cursor>, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved(event) => {
                events::dispatch_cursor_moved_events(&self.command_tx, cursor, &self.events, event)
            }
            WindowEvent::MouseButtonInput(event) => events::dispatch_mouse_button_input_events(
                &self.command_tx,
                cursor,
                &self.events,
                event,
            ),
            WindowEvent::MouseWheel(event) => {
                events::dispatch_mouse_wheel_events(&self.command_tx, cursor, &self.events, event)
            }
            WindowEvent::ReceivedCharacter(event) => events::dispatch_received_character_events(
                &self.command_tx,
                cursor,
                &self.events,
                event,
            ),
            WindowEvent::KeyboardInput(event) => events::dispatch_keyboard_input_events(
                &self.command_tx,
                cursor,
                &self.events,
                event,
            ),
            _ => (),
        }
    }

    pub fn run(&mut self, device: &Device, queue: &Queue) {
        for (id, doc) in self.windows.iter_mut() {
            let tree = self.render.get_mut(*id).unwrap();
            let events = self.events.get_mut(id).unwrap();
            events::update_events_from_layout_tree(tree, events);

            doc.run_effects();
            doc.flush_node_queue(tree, events);
        }

        self.render.update(device, queue);

        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                WindowCommand::Close(_) => {
                    todo!()
                }
                WindowCommand::SetCursorIcon(id, icon) => {
                    todo!()
                }
                WindowCommand::SetTitle(id, title) => todo!(),
            }
        }
    }
}
