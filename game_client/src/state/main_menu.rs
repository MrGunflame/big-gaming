use game_common::components::Transform;
use game_common::components::{Color, MeshInstance, PointLight};
use game_common::world::World;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::CameraId;
use game_render::Renderer;
use game_window::windows::WindowId;
use glam::Vec3;

#[derive(Debug)]
pub struct MainMenuState {
    camera: CameraId,
}

impl MainMenuState {
    pub fn new(renderer: &mut Renderer, window_id: WindowId, world: &mut World) -> Self {
        let camera = renderer.entities.cameras.insert(Camera {
            transform: Transform {
                translation: Vec3::new(10.0, 0.0, 0.0),
                ..Default::default()
            },
            projection: Projection::default(),
            target: RenderTarget::Window(window_id),
        });

        let obj = world.spawn();
        world.insert_typed(obj, Transform::default());
        world.insert_typed(
            obj,
            MeshInstance {
                path: "sponza.glb".into(),
            },
        );

        let light = world.spawn();
        world.insert_typed(
            light,
            Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
        );
        world.insert_typed(
            light,
            PointLight {
                color: Color::WHITE,
                intensity: 70.0,
                radius: 100.0,
            },
        );

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
