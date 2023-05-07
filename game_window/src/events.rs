use bevy_ecs::prelude::Entity;
use glam::Vec2;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowDestroyed {
    pub window: Entity,
}

/// A event fired when the cursor moved inside a window.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CursorMoved {
    pub window: Entity,
    // FIXME: Should this be a UVec2 instead?
    pub position: Vec2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CursorEntered {
    pub window: Entity,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CursorLeft {
    pub window: Entity,
}
