use game_common::components::actor::ActorProperties;
use game_common::components::transform::Transform;
use glam::{Mat3, Vec3};

#[derive(Clone, Debug, Default)]
pub struct CameraController {
    pub transform: Transform,
    pub mode: CameraMode,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson {
        distance: f32,
    },
    Detached,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            mode: CameraMode::FirstPerson,
        }
    }

    pub fn sync_with_entity(&mut self, entity: Transform, props: ActorProperties) {
        match self.mode {
            CameraMode::FirstPerson => {
                self.transform.translation = entity.translation + props.eyes;
                self.transform.rotation = entity.rotation;
            }
            CameraMode::ThirdPerson { distance } => {
                let dir = props.rotation * -Vec3::Z;

                self.transform.translation = entity.translation + -(dir * distance);
                self.transform.rotation = props.rotation;
            }
            // Don't sync in detached mode.
            CameraMode::Detached => (),
        }
    }
}
