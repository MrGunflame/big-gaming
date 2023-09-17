use game_common::components::transform::Transform;
use game_common::math::Ray;
use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseMotion};
use game_render::aabb::Aabb;
use game_render::camera::{Camera, Projection};
use game_render::entities::ObjectId;
use game_window::cursor::Cursor;
use game_window::events::VirtualKeyCode;
use glam::{Quat, Vec2, Vec3};

// pub fn handle_selection_input(
//     mut events: EventReader<MouseButtonInput>,
//     cursor: Res<Cursor>,
//     windows: Query<&WindowState>,
//     cameras: Query<(&Transform, &Camera)>,
//     meshes: Query<(Entity, &Aabb)>,
//     mut selection: ResMut<Selection>,
// ) {
//     for event in events.iter() {
//         if !event.button.is_left() || !event.state.is_pressed() {
//             continue;
//         }

//         let window = cursor.window().unwrap();
//         let window = windows.get(window).unwrap();

//         let size = window.inner_size();
//         let cursor_pos = cursor.position();

//         let Ok((transform, camera)) = cameras.get_single() else {
//             return;
//         };

//         let ray = camera.viewport_to_world(
//             *transform,
//             Vec2::new(size.width as f32, size.height as f32),
//             cursor_pos,
//         );

//         for (entity, aabb) in &meshes {
//             if hit_test(ray, *aabb) {
//                 selection.entities = vec![entity];
//             }
//         }
//     }
// }

// pub fn update_edit_mode(
//     mut events: EventReader<KeyboardInput>,
//     mut selection: ResMut<Selection>,
//     cursor: Res<Cursor>,
// ) {
//     for event in events.iter() {
//         match event.key_code {
//             Some(VirtualKeyCode::G) => {
//                 selection.edit_mode = EditMode::Translate;
//                 selection.start = cursor.position();
//             }
//             Some(VirtualKeyCode::R) => {
//                 selection.edit_mode = EditMode::Rotate;
//                 selection.start = cursor.position();
//             }
//             Some(VirtualKeyCode::S) => {
//                 selection.edit_mode = EditMode::Scale;
//                 selection.start = cursor.position();
//             }
//             _ => (),
//         }
//     }
// }

// pub fn update_selection_transform(
//     mut events: EventReader<MouseMotion>,
//     mut selection: ResMut<Selection>,
//     mut entities: Query<(&mut Transform), Without<Camera>>,
//     mut cameras: Query<(&Transform, &Camera)>,
//     windows: Query<&WindowState>,
//     cursor: Res<Cursor>,
// ) {
//     let Ok((camera_transform, camera)) = cameras.get_single() else {
//         return;
//     };

//     let Some(window) = cursor.window() else {
//         return;
//     };
//     let window = windows.get(window).unwrap();
//     let viewport_size = window.inner_size();
//     let cursor_pos = cursor.position();

//     for event in events.iter() {
//         let start = camera.viewport_to_world(
//             *camera_transform,
//             Vec2::new(viewport_size.width as f32, viewport_size.height as f32),
//             selection.start,
//         );
//         let end = camera.viewport_to_world(
//             *camera_transform,
//             Vec2::new(viewport_size.width as f32, viewport_size.height as f32),
//             cursor_pos,
//         );

//         match selection.edit_mode {
//             EditMode::None => (),
//             EditMode::Translate => {
//                 for entity in &selection.entities {
//                     let mut transform = entities.get_mut(*entity).unwrap();
//                     //transform.translation += 1.0;
//                     transform.translation += end.origin - start.origin;
//                 }
//             }
//             EditMode::Rotate => {
//                 for entity in &selection.entities {
//                     let mut transform = entities.get_mut(*entity).unwrap();
//                     transform.rotation = Quat::IDENTITY;
//                 }
//             }
//             EditMode::Scale => {
//                 for entity in &selection.entities {
//                     let mut transform = entities.get_mut(*entity).unwrap();
//                     transform.scale += 0.1;
//                 }
//             }
//         }
//     }
// }

pub fn hit_test(ray: Ray, aabb: Aabb) -> bool {
    for normal in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
        let origin = aabb.center + aabb.half_extents * normal;

        if let Some(point) = ray.plane_intersection(origin, normal) {
            if point.x >= aabb.min().x
                && point.x <= aabb.max().x
                && point.y >= aabb.min().y
                && point.y <= aabb.max().y
                && point.z >= aabb.min().z
                && point.z <= aabb.max().z
            {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use game_render::aabb::Aabb;
    use glam::Vec3;

    use super::{hit_test, Ray};

    #[test]
    fn hit_test_hit() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 1.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        };

        let aabb = Aabb {
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(50.0, 50.0, 0.0),
        };

        assert!(hit_test(ray, aabb));
    }

    #[test]
    fn hit_test_miss() {
        let ray = Ray {
            origin: Vec3::new(51.0, 0.0, 1.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        };

        let aabb = Aabb {
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(50.0, 50.0, 0.0),
        };

        assert!(!hit_test(ray, aabb));
    }

    #[test]
    fn hit_test_miss_parallel() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 1.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
        };

        let aabb = Aabb {
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(50.0, 50.0, 0.0),
        };

        assert!(!hit_test(ray, aabb));
    }

    #[test]
    fn hit_test_hit_contained() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 0.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
        };

        let aabb = Aabb {
            center: Vec3::new(0.0, 0.0, 0.0),
            half_extents: Vec3::new(50.0, 50.0, 0.0),
        };

        assert!(hit_test(ray, aabb));
    }
}
