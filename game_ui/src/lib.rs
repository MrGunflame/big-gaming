//! UI related systems

use std::collections::HashMap;
use std::sync::{mpsc, Arc};

pub mod events;
pub mod layout;
pub mod primitive;
pub mod reactive;
pub mod render;
pub mod style;
// pub mod widgets;

use events::{Events, WindowCommand};
use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tracing::trace_span;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use glam::UVec2;
use reactive::Runtime;

use render::UiRenderer;

pub struct UiState {
    renderer: UiRenderer,
    events: HashMap<RenderTarget, Events>,
    runtime: Runtime,
    command_rx: mpsc::Receiver<WindowCommand>,
    command_tx: mpsc::Sender<WindowCommand>,
}

impl UiState {
    pub fn new(renderer: &Renderer) -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        Self {
            renderer: UiRenderer::new(renderer),
            runtime: Runtime::new(),
            events: HashMap::new(),
            command_rx,
            command_tx,
        }
    }

    pub fn runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    pub fn create(&mut self, target: RenderTarget, size: UVec2) {
        self.renderer.insert(target, size);
        self.runtime.create_window(target, size);
        self.events.insert(target, Events::new());
    }

    pub fn resize(&mut self, target: RenderTarget, size: UVec2) {
        self.renderer.resize(target, size);
    }

    pub fn destroy(&mut self, target: RenderTarget) {
        self.renderer.remove(target);
        self.runtime.destroy_window(target);
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

    pub fn update(&mut self, cmds: &mut Vec<WindowCommand>) {
        let _span = trace_span!("UiState::update");

        let rt = self.runtime.inner.lock();
        let mut docs = Vec::new();
        for (id, window) in rt.windows.iter() {
            for doc in &window.documents {
                docs.push((*doc, id));
            }
        }

        for (doc, win) in docs {
            let doc = rt.documents.get(doc.0).unwrap();

            let tree = self.renderer.get_mut(*win).unwrap();
            *tree = doc.layout.clone();
        }

        // for (id, doc) in self.targets.iter_mut() {
        //     let tree = self.renderer.get_mut(*id).unwrap();
        //     let events = self.events.get_mut(id).unwrap();
        //     events::update_events_from_layout_tree(tree, events);

        //     doc.run_effects();
        //     doc.flush_node_queue(tree, events);
        // }

        self.renderer.update();

        while let Ok(cmd) = self.command_rx.try_recv() {
            cmds.push(cmd);
        }
    }
}
