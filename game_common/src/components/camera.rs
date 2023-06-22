use bevy_ecs::prelude::Component;

#[derive(Copy, Clone, Debug, Default, PartialEq, Component)]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson {
        distance: f32,
    },
    /// Camera detached from player
    Detached,
}

impl CameraMode {
    pub const fn is_first_person(self) -> bool {
        matches!(self, Self::FirstPerson)
    }

    pub const fn is_third_person(self) -> bool {
        matches!(self, Self::ThirdPerson { distance: _ })
    }

    pub const fn is_detached(self) -> bool {
        matches!(self, Self::Detached)
    }
}
