use game_common::components::{Color, GlobalTransform, MeshInstance, PointLight};
use game_common::components::{PrimaryCamera, Transform};
use game_common::entity::EntityId;
use game_common::world::World;
use game_core::time::Time;
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
            GlobalTransform(Transform {
                translation: Vec3::new(10.0, 0.0, 0.0),
                ..Default::default()
            }),
        );
        world.insert_typed(camera, PrimaryCamera);

        // let res = world.insert_resource(include_bytes!("../../sponza.glb").to_vec().into());

        let obj = world.spawn();
        world.insert_typed(obj, GlobalTransform::default());
        // world.insert_typed(obj, MeshInstance { model: res.into() });

        let light = world.spawn();
        world.insert_typed(
            light,
            GlobalTransform(Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            }),
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

    pub fn update(&mut self, time: &mut Time, world: &mut World) {
        let delta = time.delta();

        let mut transform = world.get_typed::<GlobalTransform>(self.camera).unwrap();

        //camera.transform.translation.x = 10.0;
        //camera.transform.translation.z = 1.0;
        transform.0.translation.y += 0.001 * delta.as_secs_f32() * 60.0;
        transform.0 = transform.0.looking_at(Vec3::ZERO, Vec3::Y);

        if transform.0.translation.y > 2.1 {
            transform.0.translation.y = 0.0;
        }

        world.insert_typed(self.camera, transform);
    }
}
