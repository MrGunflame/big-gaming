//! UI related systems

use std::sync::Arc;

mod clipboard;
pub mod events;
pub mod layout;
pub mod primitive;
pub mod reactive;
pub mod render;
pub mod runtime;
pub mod style;
pub mod widgets;

use game_render::camera::RenderTarget;
use game_render::Renderer;
use game_tracing::trace_span;
use game_window::cursor::Cursor;
use game_window::events::WindowEvent;
use glam::UVec2;

use render::UiRenderer;
use runtime::Runtime;

pub struct UiState {
    renderer: UiRenderer,
    runtime: Runtime,
}

impl UiState {
    pub fn new(renderer: &Renderer) -> Self {
        Self {
            renderer: UiRenderer::new(renderer),
            runtime: Runtime::new(),
        }
    }

    pub fn runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    pub fn create(&mut self, target: RenderTarget, props: WindowProperties) {
        self.renderer.insert(target, props.size, props.scale_factor);
        self.runtime.windows().create(target, props);
    }

    pub fn resize(&mut self, target: RenderTarget, size: UVec2) {
        self.renderer.resize(target, size);
        self.runtime.windows().update_size(target, size);
    }

    pub fn update_scale_factor(&mut self, target: RenderTarget, scale_factor: f64) {
        self.renderer.update_scale_factor(target, scale_factor);
        self.runtime
            .windows()
            .update_scale_factor(target, scale_factor);
    }

    pub fn destroy(&mut self, target: RenderTarget) {
        self.renderer.remove(target);
        self.runtime.windows().destroy(target);
    }

    pub fn send_event(&mut self, cursor: &Arc<Cursor>, event: WindowEvent) {
        let Some(window) = cursor.window() else {
            return;
        };
        *self.runtime.cursor.lock() = Some(cursor.clone());

        match event {
            WindowEvent::CursorMoved(event) => {
                self.runtime.send_event(event);
            }
            WindowEvent::MouseButtonInput(event) => {
                self.runtime.send_event(event);
            }
            WindowEvent::MouseWheel(event) => {
                self.runtime.send_event(event);
            }
            WindowEvent::KeyboardInput(event) => {
                self.runtime.send_event(event);
            }
            _ => (),
        }
    }

    pub fn update(&mut self) {
        let _span = trace_span!("UiState::update").entered();

        let mut rt = self.runtime.inner.lock();
        for doc in rt.documents.values_mut() {
            doc.tree.compute_layout();
            let nodes = doc.tree.collect_all();

            let tree = self.renderer.get_mut(doc.window).unwrap();
            *tree = nodes;
        }

        self.renderer.update();
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WindowProperties {
    pub size: UVec2,
    pub scale_factor: f64,
}
