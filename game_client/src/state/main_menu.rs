use bevy_ecs::prelude::Component;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, ResMut};
use game_common::bundles::TransformBundle;
use game_common::components::transform::Transform;
use game_render::camera::Camera;
use game_render::color::Color;
use game_render::light::{PointLight, PointLightBundle};
use game_scene::{SceneBundle, Scenes};
use glam::{Quat, Vec3};

pub fn setup_main_scene(mut commands: Commands, mut scenes: ResMut<Scenes>) {
    commands.spawn(SceneBundle {
        scene: scenes.load("sponza.model"),
        transform: TransformBundle::default(),
    });

    commands.spawn(PointLightBundle {
        light: PointLight {
            color: Color::WHITE,
            intensity: 70.0,
            radius: 100.0,
        },
        transform: TransformBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    });
}

pub fn move_camera(mut cameras: Query<&mut Transform, With<Camera>>) {
    for mut camera in &mut cameras {
        camera.translation.x = 10.0;
        camera.translation.z = 1.0;
        camera.translation.y += 0.001;
        *camera = camera.looking_at(Vec3::ZERO, Vec3::Y);

        if camera.translation.y > 2.1 {
            camera.translation.y = 0.0;
        }
        //camera.translation += Vec3::new(0.0, 0.01, 0.0);
    }
}
