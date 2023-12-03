use game_common::components::transform::Transform;
use game_core::hierarchy::TransformHierarchy;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::entities::CameraId;
use game_render::light::PointLight;
use game_render::Renderer;
use game_scene::scene2::Node;
use game_window::windows::WindowId;
use glam::Vec3;

use crate::scene::SceneState;

#[derive(Debug)]
pub struct MainMenuState {
    camera: CameraId,
}

impl MainMenuState {
    pub fn new(
        scenes: &mut SceneState,
        renderer: &mut Renderer,
        window_id: WindowId,
        hierarchy: &mut TransformHierarchy,
    ) -> Self {
        let camera = renderer.entities.cameras.insert(Camera {
            transform: Transform {
                translation: Vec3::new(10.0, 0.0, 0.0),
                ..Default::default()
            },
            projection: Projection::default(),
            target: RenderTarget::Window(window_id),
        });

        let key = scenes.graph.append(
            None,
            Node {
                transform: Transform::default(),
                components: vec![],
            },
        );
        scenes.spawner.spawn(key, "sponza.model");

        renderer.entities.point_lights.insert(PointLight {
            transform: Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
            color: Color::WHITE,
            intensity: 70.0,
            radius: 100.0,
        });

        Self { camera }
    }

    pub fn update(&mut self, renderer: &mut Renderer) {
        let mut camera = renderer.entities.cameras.get_mut(self.camera).unwrap();

        //camera.transform.translation.x = 10.0;
        //camera.transform.translation.z = 1.0;
        camera.transform.translation.y += 0.001;
        camera.transform = camera.transform.looking_at(Vec3::ZERO, Vec3::Y);

        if camera.transform.translation.y > 2.1 {
            camera.transform.translation.y = 0.0;
        }
    }
}
