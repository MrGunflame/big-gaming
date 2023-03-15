use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::{
    Camera3d, Camera3dBundle, Commands, Component, EventReader, Input, KeyCode, Mat3, MouseButton,
    Plugin, Quat, Query, Res, Transform, Vec3, With,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CameraPlugin;

#[derive(Copy, Clone, Debug, Default, PartialEq, Component)]
pub struct CameraOrigin {
    origin: Vec3,
    distance: f32,
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup_camera)
            .add_system(camera_inputs)
            .add_system(camera_zoom);
    }
}

fn setup_camera(mut commands: Commands) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(CameraOrigin {
            origin: Vec3::splat(0.0),
            distance: 0.0,
        });
}

fn camera_inputs(
    mouse: Res<Input<MouseButton>>,
    mut events: EventReader<MouseMotion>,
    keyboard: Res<Input<KeyCode>>,
    mut cameras: Query<(&mut Transform, &mut CameraOrigin), With<Camera3d>>,
) {
    let (mut transform, mut camera) = cameras.single_mut();

    if !mouse.pressed(MouseButton::Middle) {
        return;
    }

    if keyboard.pressed(KeyCode::LShift) | keyboard.pressed(KeyCode::RShift) {
        for event in events.iter() {
            let x = event.delta.x * 0.01;
            let y = event.delta.y * 0.01;

            camera.origin += transform.translation * Vec3::new(x, y, 0.0);
        }

        let rot_mat = Mat3::from_quat(transform.rotation);
        transform.translation = camera.origin + rot_mat * Vec3::new(0.0, 0.0, camera.distance);

        return;
    }

    for event in events.iter() {
        let x = event.delta.x * 0.01;
        let y = event.delta.y * 0.01;

        // let dir = transform.rotation.dir_vec();

        // let rot = Quat::from_axis_angle(dir, event.delta.x);

        let q1 = Quat::from_axis_angle(Vec3::Y, -x);
        let q2 = Quat::from_axis_angle(Vec3::X, -y);

        transform.rotation = q1 * transform.rotation;
        transform.rotation = transform.rotation * q2;
    }

    let rot_mat = Mat3::from_quat(transform.rotation);
    transform.translation = camera.origin + rot_mat * Vec3::new(0.0, 0.0, camera.distance);
}

fn camera_zoom(
    mut events: EventReader<MouseWheel>,
    mut cameras: Query<(&mut Transform, &mut CameraOrigin), With<Camera3d>>,
) {
    let (mut transform, mut camera) = cameras.single_mut();

    for event in events.iter() {
        camera.distance -= event.y;

        let rot_mat = Mat3::from_quat(transform.rotation);
        transform.translation = camera.origin + rot_mat * Vec3::new(0.0, 0.0, camera.distance);
    }
}
