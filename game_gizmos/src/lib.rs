//! API for immediate mode 3D drawing for debugging purposes.
//!

mod render;

use std::sync::Arc;

use game_common::components::Color;
use game_render::camera::Camera;
use game_render::Renderer;
use game_tracing::trace_span;
use glam::Vec3;
use parking_lot::Mutex;
use parking_lot::RwLock;
use render::pipeline::GizmoPass;
use render::DrawCommand;

/// A immediate mode gizmo renderer.
#[derive(Debug)]
pub struct Gizmos {
    camera: Arc<Mutex<Option<Camera>>>,
    /// Elements that are currently being rendered.
    current: Arc<RwLock<Vec<DrawCommand>>>,
    /// Elements queued for the next render submission.
    next: Mutex<Vec<DrawCommand>>,
}

impl Gizmos {
    /// Creates a new `Gizmos` renderer.
    pub fn new(renderer: &Renderer) -> Self {
        let elements = Arc::new(RwLock::new(Vec::new()));
        let camera = Arc::new(Mutex::new(None));

        let node = GizmoPass::new(renderer.device(), elements.clone(), camera.clone());
        renderer.add_to_graph(node);

        Self {
            current: elements,
            camera,
            next: Mutex::new(Vec::new()),
        }
    }

    /// Draw a line from `start` to `end`.
    pub fn line(&self, start: Vec3, end: Vec3, color: Color) {
        self.next.lock().push(DrawCommand { start, end, color });
    }

    /// Update the camera position from which the gizmo renderer draws 3D objects.
    pub fn update_camera(&self, camera: Camera) {
        *self.camera.lock() = Some(camera);
    }

    /// Swap the draw buffers, submitting all commands since the last call to `swap_buffers` to
    /// the renderer.
    pub fn swap_buffers(&self) {
        let _span = trace_span!("Gizmos::swap_buffers").entered();

        // Swap the `current` and `next` buffers in place without deallocating
        // the memory for either Vec.
        let mut next = self.next.lock();
        let mut current = self.current.write();
        std::mem::swap(&mut *next, &mut *current);

        next.clear();
    }
}
