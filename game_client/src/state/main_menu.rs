use game_common::components::{Color, MeshInstance, PointLight};
use game_common::components::{PrimaryCamera, Transform};
use game_common::entity::EntityId;
use game_common::world::World;
use glam::Vec3;

#[derive(Debug)]
pub struct MainMenuState {
    camera: EntityId,
}

impl MainMenuState {
    pub fn new(world: &mut World) -> Self {
        let camera = world.spawn();
        world.insert_typed(
            camera,
            Transform {
                translation: Vec3::new(10.0, 0.0, 0.0),
                ..Default::default()
            },
        );
        world.insert_typed(camera, PrimaryCamera);

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

    pub fn update(&mut self, world: &mut World) {
        let mut transform = world.get_typed::<Transform>(self.camera).unwrap();

        //camera.transform.translation.x = 10.0;
        //camera.transform.translation.z = 1.0;
        transform.translation.y += 0.001;
        transform = transform.looking_at(Vec3::ZERO, Vec3::Y);

        if transform.translation.y > 2.1 {
            transform.translation.y = 0.0;
        }

        world.insert_typed(self.camera, transform);
    }
}
