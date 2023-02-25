use bevy_ecs::prelude::Component;

#[derive(Copy, Clone, Debug, Default, PartialEq, Component)]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson {
        distance: f32,
    },
}
