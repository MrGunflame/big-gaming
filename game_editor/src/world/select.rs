use bevy_ecs::prelude::EventReader;
use bevy_ecs::query::With;
use bevy_ecs::system::{Query, Res};
use game_common::components::transform::Transform;
use game_common::math::Ray;
use game_input::mouse::MouseButtonInput;
use game_render::aabb::Aabb;
use game_render::camera::Camera;
use game_window::cursor::Cursor;
use game_window::WindowState;
use glam::{Vec2, Vec3};

pub fn handle_selection_input(
    mut events: EventReader<MouseButtonInput>,
    cursor: Res<Cursor>,
    windows: Query<&WindowState>,
    cameras: Query<(&Transform, &Camera)>,
    meshes: Query<&Aabb>,
) {
    for event in events.iter() {
        if !event.button.is_left() || !event.state.is_pressed() {
            continue;
        }

        let window = cursor.window().unwrap();
        let window = windows.get(window).unwrap();

        let size = window.inner_size();
        let cursor_pos = cursor.position();

        let Ok((transform, camera)) = cameras.get_single() else {
            return;
        };

        let ray = camera.viewport_to_world(
            *transform,
            Vec2::new(size.width as f32, size.height as f32),
            cursor_pos,
        );
        dbg!(ray);

        for aabb in &meshes {
            if hit_test(ray, *aabb) {
                dbg!("yes");
            }
        }
    }
}

fn hit_test(ray: Ray, aabb: Aabb) -> bool {
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
