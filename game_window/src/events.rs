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
pub struct WindowClosing {
    pub window: Entity,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowDestroyed {
    pub window: Entity,
}

/// A event fired when the cursor moved inside a window.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CursorMoved {
    pub window: Entity,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReceivedCharacter {
    pub window: Entity,
    pub char: char,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowCloseRequested {
    pub window: Entity,
}

// FIXME: Export a custom type from input crate.
pub use winit::event::VirtualKeyCode;
