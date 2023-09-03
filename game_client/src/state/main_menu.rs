use game_common::components::transform::Transform;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::color::Color;
use game_render::light::PointLight;
use game_render::RenderState;
use game_scene::Scenes;
use game_window::windows::WindowId;
use glam::Vec3;

pub fn setup_main_scene(scenes: &mut Scenes, renderer: &mut RenderState, window_id: WindowId) {
    renderer.entities.push_camera(Camera {
        transform: Transform {
            translation: Vec3::new(10.0, 0.0, 1.0),
            ..Default::default()
        },
        projection: Projection::default(),
        target: RenderTarget::Window(window_id),
    });

    // let handle = scenes.load("sponza.model");
    // std::mem::forget(handle);

    renderer.entities.push_point_light(PointLight {
        transform: Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        color: Color::WHITE,
        intensity: 70.0,
        radius: 100.0,
    });
}

// pub fn move_camera(
//     state: Res<InternalGameState>,
//     mut cameras: Query<&mut Transform, With<Camera>>,
// ) {
//     if state.state != GameState::MainMenu {
//         return;
//     }

//     for mut camera in &mut cameras {
//         camera.translation.x = 10.0;
//         camera.translation.z = 1.0;
//         camera.translation.y += 0.001;
//         *camera = camera.looking_at(Vec3::ZERO, Vec3::Y);

//         if camera.translation.y > 2.1 {
//             camera.translation.y = 0.0;
//         }
//     }
// }
