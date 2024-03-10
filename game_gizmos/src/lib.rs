mod render;

use std::sync::Arc;

use game_common::components::Color;
use game_render::camera::Camera;
use game_render::Renderer;
use glam::Vec3;
use parking_lot::Mutex;
use parking_lot::RwLock;
use render::pipeline::GizmoPass;
use render::DrawCommand;

#[derive(Debug)]
pub struct Gizmos {
    elements: Arc<RwLock<Vec<DrawCommand>>>,
    camera: Arc<Mutex<Option<Camera>>>,
}

impl Gizmos {
    pub fn new(renderer: &Renderer) -> Self {
        let elements = Arc::new(RwLock::new(Vec::new()));
        let camera = Arc::new(Mutex::new(None));

        let node = GizmoPass::new(renderer.device(), elements.clone(), camera.clone());
        renderer.add_to_graph(node);

        Self { elements, camera }
    }

    pub fn line(&self, start: Vec3, end: Vec3, color: Color) {
        self.elements
            .write()
            .push(DrawCommand { start, end, color });
    }

    pub fn update_camera(&self, camera: Camera) {
        *self.camera.lock() = Some(camera);
    }

    pub fn clear(&self) {
        self.elements.write().clear();
    }
}
