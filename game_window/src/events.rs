use bevy_ecs::prelude::Entity;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowCreated {
    pub window: Entity,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowResized {
    pub window: Entity,
    pub width: u32,
    pub height: u32,
}
