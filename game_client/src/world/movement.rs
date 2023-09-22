use game_common::components::transform::Transform;
use game_input::mouse::MouseMotion;
use glam::{Quat, Vec3};

pub fn update_rotation(mut transform: Transform, event: MouseMotion) -> Transform {
    let yaw = event.delta.x * 0.001;
    let pitch = event.delta.y * 0.001;

    let q1 = Quat::from_axis_angle(Vec3::Y, -yaw);
    let q2 = Quat::from_axis_angle(Vec3::X, -pitch);

    transform.rotation = q1 * transform.rotation;
    transform.rotation = transform.rotation * q2;
    transform
}
