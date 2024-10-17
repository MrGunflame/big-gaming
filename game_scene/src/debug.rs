use std::f32::consts::PI;

use game_common::components::{Axis, Collider, ColliderShape, Color, Transform};
use game_common::world::{QueryWrapper, World};
use game_gizmos::Gizmos;
use game_tracing::trace_span;
use glam::{Quat, Vec3};

/// Draw debugging lines for all colliders.
pub fn draw_collider_lines(world: &World, gizmos: &Gizmos) {
    let _span = trace_span!("draw_collider_lines").entered();

    for (_, QueryWrapper((mut transform, collider))) in
        world.query::<QueryWrapper<(Transform, Collider)>>()
    {
        // Colliders don't use scale and always use the default scale
        // value of 1. (The physics engine cannot efficiently support
        // certain transformations like shearing.)
        transform.scale = Vec3::ONE;

        match collider.shape {
            ColliderShape::Cuboid(cuboid) => {
                let min_x = -cuboid.hx;
                let max_x = cuboid.hx;
                let min_y = -cuboid.hy;
                let max_y = cuboid.hy;
                let min_z = -cuboid.hz;
                let max_z = cuboid.hz;

                let lines = [
                    // Front-Back
                    [
                        Vec3::new(min_x, min_y, min_z),
                        Vec3::new(min_x, min_y, max_z),
                    ],
                    [
                        Vec3::new(min_x, max_y, min_z),
                        Vec3::new(min_x, max_y, max_z),
                    ],
                    [
                        Vec3::new(max_x, min_y, min_z),
                        Vec3::new(max_x, min_y, max_z),
                    ],
                    [
                        Vec3::new(max_x, max_y, min_z),
                        Vec3::new(max_x, max_y, max_z),
                    ],
                    // Bottom-Top
                    [
                        Vec3::new(min_x, min_y, min_z),
                        Vec3::new(min_x, max_y, min_z),
                    ],
                    [
                        Vec3::new(min_x, min_y, max_z),
                        Vec3::new(min_x, max_y, max_z),
                    ],
                    [
                        Vec3::new(max_x, min_y, min_z),
                        Vec3::new(max_x, max_y, min_z),
                    ],
                    [
                        Vec3::new(max_x, min_y, max_z),
                        Vec3::new(max_x, max_y, max_z),
                    ],
                    // Left-Right
                    [
                        Vec3::new(min_x, min_y, min_z),
                        Vec3::new(max_x, min_y, min_z),
                    ],
                    [
                        Vec3::new(min_x, min_y, max_z),
                        Vec3::new(max_x, min_y, max_z),
                    ],
                    [
                        Vec3::new(min_x, max_y, min_z),
                        Vec3::new(max_x, max_y, min_z),
                    ],
                    [
                        Vec3::new(min_x, max_y, max_z),
                        Vec3::new(max_x, max_y, max_z),
                    ],
                ];

                for [start, end] in lines {
                    gizmos.line(
                        transform.transform_point(start),
                        transform.transform_point(end),
                        Color::RED,
                    );
                }
            }
            ColliderShape::Ball(ball) => {
                gizmos.sphere(transform.translation, ball.radius, Color::RED);
            }
            ColliderShape::Capsule(capsule) => {
                // Top "circle" section of the capsule.
                for rotation in [
                    Quat::from_axis_angle(Vec3::X, PI / 2.0),
                    Quat::from_axis_angle(Vec3::X, PI / 2.0)
                        * Quat::from_axis_angle(Vec3::Z, PI / 2.0),
                ] {
                    gizmos.arc(
                        transform.translation + capsule.axis.to_vec3() * capsule.half_height,
                        rotation,
                        PI,
                        capsule.radius,
                        Color::RED,
                    );
                }

                // Bottom "circle" section of the capsule.
                for rotation in [
                    Quat::from_axis_angle(Vec3::X, -PI / 2.0),
                    Quat::from_axis_angle(Vec3::X, -PI / 2.0)
                        * Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
                ] {
                    gizmos.arc(
                        transform.translation - capsule.axis.to_vec3() * capsule.half_height,
                        rotation,
                        PI,
                        capsule.radius,
                        Color::RED,
                    );
                }

                for translation in [
                    transform.translation + capsule.axis.to_vec3() * capsule.half_height,
                    transform.translation,
                    transform.translation - capsule.axis.to_vec3() * capsule.half_height,
                ] {
                    gizmos.circle(
                        translation,
                        capsule.axis.to_vec3(),
                        capsule.radius,
                        Color::RED,
                    );
                }

                let center_points = match capsule.axis {
                    Axis::X => [
                        transform.translation + Vec3::Y * capsule.radius,
                        transform.translation - Vec3::Y * capsule.radius,
                        transform.translation + Vec3::Z * capsule.radius,
                        transform.translation - Vec3::Z * capsule.radius,
                    ],
                    Axis::Y => [
                        transform.translation + Vec3::X * capsule.radius,
                        transform.translation - Vec3::X * capsule.radius,
                        transform.translation + Vec3::Z * capsule.radius,
                        transform.translation - Vec3::Z * capsule.radius,
                    ],
                    Axis::Z => [
                        transform.translation + Vec3::X * capsule.radius,
                        transform.translation - Vec3::X * capsule.radius,
                        transform.translation + Vec3::Y * capsule.radius,
                        transform.translation - Vec3::Y * capsule.radius,
                    ],
                };

                for point in center_points {
                    gizmos.line(
                        point + capsule.axis.to_vec3() * capsule.half_height,
                        point - capsule.axis.to_vec3() * capsule.half_height,
                        Color::RED,
                    );
                }
            }
            ColliderShape::TriMesh(mesh) => {
                for indices in mesh.indices().windows(3) {
                    let a = mesh.vertices()[indices[0] as usize];
                    let b = mesh.vertices()[indices[1] as usize];
                    let c = mesh.vertices()[indices[2] as usize];

                    gizmos.line(a, b, Color::RED);
                    gizmos.line(b, c, Color::RED);
                    gizmos.line(c, a, Color::RED);
                }
            }
        }
    }
}
