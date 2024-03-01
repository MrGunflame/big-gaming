use game_common::components::actor::ActorProperties;
use game_common::components::Transform;
use glam::Vec3;

const SPEED_MULTIPLIER: f32 = 0.1;

#[derive(Clone, Debug, Default)]
pub struct CameraController {
    pub transform: Transform,
    pub mode: CameraMode,
    pub detached_state: DetachedState,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DetachedState {
    pub forward: bool,
    pub left: bool,
    pub right: bool,
    pub back: bool,
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

impl CameraMode {}

impl CameraController {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            mode: CameraMode::FirstPerson,
            detached_state: DetachedState::default(),
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

    pub fn update(&mut self) {
        if self.mode != CameraMode::Detached {
            return;
        }

        if self.detached_state.forward {
            self.transform.translation += self.transform.rotation * -Vec3::Z * SPEED_MULTIPLIER;
        }

        if self.detached_state.back {
            self.transform.translation += self.transform.rotation * Vec3::Z * SPEED_MULTIPLIER;
        }

        if self.detached_state.left {
            self.transform.translation += self.transform.rotation * -Vec3::X * SPEED_MULTIPLIER;
        }

        if self.detached_state.right {
            self.transform.translation += self.transform.rotation * Vec3::X * SPEED_MULTIPLIER;
        }
    }
}
