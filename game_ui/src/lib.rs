//! UI related systems

use std::collections::HashMap;
use std::sync::{mpsc, Arc};

pub mod events;
pub mod layout;
pub mod reactive;
pub mod render;
pub mod style;
pub mod widgets;

use events::{Events, WindowCommand};
use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tracing::trace_span;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use game_window::windows::Windows;
use glam::UVec2;
use reactive::{Document, Runtime};

use render::UiRenderer;

pub struct UiState {
    renderer: UiRenderer,
    targets: HashMap<RenderTarget, Document>,
    events: HashMap<RenderTarget, Events>,
    pub runtime: Runtime,
    command_rx: mpsc::Receiver<WindowCommand>,
    command_tx: mpsc::Sender<WindowCommand>,
}

impl UiState {
    pub fn new(renderer: &Renderer) -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        Self {
            renderer: UiRenderer::new(renderer),
            runtime: Runtime::new(),
            targets: HashMap::new(),
            events: HashMap::new(),
            command_rx,
            command_tx,
        }
    }

    pub fn create(&mut self, target: RenderTarget, size: UVec2) {
        self.renderer.insert(target, size);
        self.targets
            .insert(target, Document::new(self.runtime.clone()));
        self.events.insert(target, Events::new());
    }

    pub fn get_mut(&mut self, target: RenderTarget) -> Option<&mut Document> {
        self.targets.get_mut(&target)
    }

    pub fn resize(&mut self, target: RenderTarget, size: UVec2) {
        self.renderer.resize(target, size);
    }

    pub fn destroy(&mut self, target: RenderTarget) {
        self.renderer.remove(target);
        self.targets.remove(&target);
        self.events.remove(&target);
    }

    pub fn send_event(&mut self, cursor: &Arc<Cursor>, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved(event) => events::dispatch_cursor_moved_events(
                &self.command_tx,
                cursor,
                &mut self.events,
                event,
            ),
            WindowEvent::MouseButtonInput(event) => events::dispatch_mouse_button_input_events(
                &self.command_tx,
                cursor,
                &self.events,
                event,
            ),
            WindowEvent::MouseWheel(event) => {
                events::dispatch_mouse_wheel_events(&self.command_tx, cursor, &self.events, event)
            }
            WindowEvent::KeyboardInput(event) => events::dispatch_keyboard_input_events(
                &self.command_tx,
                cursor,
                &self.events,
                event,
            ),
            _ => (),
        }
    }

    pub fn run(&mut self, renderer: &Renderer, windows: &mut Windows) {
        let device = renderer.device();
        let queue = renderer.queue();

        let _span = trace_span!("UiState::update");

        for (id, doc) in self.targets.iter_mut() {
            let tree = self.renderer.get_mut(*id).unwrap();
            let events = self.events.get_mut(id).unwrap();
            events::update_events_from_layout_tree(tree, events);

            doc.run_effects();
            doc.flush_node_queue(tree, events);
        }

        self.renderer.update(device, queue);

        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                WindowCommand::Close(id) => {
                    windows.despawn(id);
                }
                WindowCommand::SetCursorIcon(id, icon) => {
                    if let Some(state) = windows.state(id) {
                        state.set_cursor_icon(icon);
                    }
                }
                WindowCommand::SetTitle(id, title) => {
                    if let Some(state) = windows.state(id) {
                        state.set_title(&title);
                    }
                }
            }
        }
    }
}
