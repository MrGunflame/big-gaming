//! API for immediate mode 3D drawing for debugging purposes.
//!

mod render;

use std::f32::consts::PI;
use std::sync::Arc;

use game_common::components::Color;
use game_render::camera::Camera;
use game_render::Renderer;
use game_tracing::trace_span;
use glam::Quat;
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

    pub fn sphere(&self, center: Vec3, radius: f32, color: Color) {
        self.circle(center, Vec3::X, radius, color);
        self.circle(center, Vec3::Y, radius, color);
        self.circle(center, Vec3::Z, radius, color);
    }

    /// Draws a circle at `center` with the given `radius`.
    ///
    /// `normal` must be normalized.
    pub fn circle(&self, center: Vec3, normal: Vec3, radius: f32, color: Color) {
        debug_assert!(normal.is_normalized());

        const SEGMENTS: u32 = 24;
        const STEP_ANGLE: f32 = (2.0 * PI) / SEGMENTS as f32;

        // This is the distance from the center of the circle to a point
        // on the perimiter of the circle.
        let forward = normal.any_orthonormal_vector() * radius;

        // The iterations are fast enough that we can keep the lock
        // for the full duration of the loop.
        let mut cmds = self.next.lock();

        for index in 0..SEGMENTS {
            let start = center + Quat::from_axis_angle(normal, STEP_ANGLE * index as f32) * forward;
            let end =
                center + Quat::from_axis_angle(normal, STEP_ANGLE * (index + 1) as f32) * forward;

            cmds.push(DrawCommand { start, end, color });
        }
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
