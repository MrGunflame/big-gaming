use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::ptr::NonNull;

use bevy_ecs::prelude::{Component, EventReader};
use bevy_ecs::query::{Added, Changed, Or};
use bevy_ecs::system::{Query, Res};
use game_input::mouse::MouseButtonInput;
use game_window::events::{CursorMoved, ReceivedCharacter};
use glam::Vec2;

use crate::cursor::Cursor;
use crate::render::layout::{Key, LayoutTree};
use crate::render::Rect;

#[derive(Default)]
pub struct EventHandlers {
    pub cursor_moved: Option<Box<dyn Fn() + Send + Sync + 'static>>,
    pub cursor_left: Option<Box<dyn Fn() + Send + Sync + 'static>>,
    pub cursor_entered: Option<Box<dyn Fn() + Send + Sync + 'static>>,
    pub mouse_button_input: Option<Box<dyn Fn(MouseButtonInput) + Send + Sync + 'static>>,
    pub received_character: Option<Box<dyn Fn(char) + Send + Sync + 'static>>,
}

impl Debug for EventHandlers {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn map_to_ptr<T: ?Sized>(e: &Option<Box<T>>) -> Option<NonNull<T>> {
            e.as_ref().map(|e| e.as_ref().into())
        }

        f.debug_struct("EventHandlers")
            .field("cursor_moved", &map_to_ptr(&self.cursor_moved))
            .field("cursor_left", &map_to_ptr(&self.cursor_left))
            .field("cursor_entered", &map_to_ptr(&self.cursor_entered))
            .field("mouse_button_input", &map_to_ptr(&self.mouse_button_input))
            .field("received_character", &map_to_ptr(&self.received_character))
            .finish()
    }
}

#[derive(Component, Default)]
pub struct Events {
    events: HashMap<Key, EventHandlers>,
    positions: Vec<(Key, Rect)>,
}

impl Events {
    pub fn insert(&mut self, key: Key, handlers: EventHandlers) {
        self.events.insert(key, handlers);
    }

    pub fn remove(&mut self, key: Key) {
        self.events.remove(&key);
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut EventHandlers> {
        self.events.get_mut(&key)
    }
}

pub fn update_events_from_layout_tree(
    mut windows: Query<
        (&mut LayoutTree, &mut Events),
        Or<(Changed<LayoutTree>, Added<LayoutTree>)>,
    >,
) {
    for (tree, mut events) in &mut windows {
        events.positions.clear();

        for (key, layout) in tree.keys().zip(tree.layouts()) {
            let position = Rect {
                min: layout.position,
                max: Vec2::new(
                    layout.position.x + layout.width,
                    layout.position.y + layout.height,
                ),
            };

            events.positions.push((key, position));
        }
    }
}

pub fn dispatch_cursor_moved_events(windows: Query<&Events>, mut events: EventReader<CursorMoved>) {
    for event in events.iter() {
        let Ok(window) = windows.get(event.window) else {
            continue;
        };

        for (key, rect) in &window.positions {
            if hit_test(*rect, event.position) {
                let Some(handlers) = window.events.get(&key) else {
                    continue;
                };

                if let Some(f) = &handlers.cursor_moved {
                    f();
                }
            }
        }
    }
}

pub fn dispatch_mouse_button_input_events(
    cursor: Res<Cursor>,
    windows: Query<&Events>,
    mut events: EventReader<MouseButtonInput>,
) {
    if events.is_empty() {
        return;
    }

    let Some(window) = cursor.window() else {
        return;
    };

    let Ok(window) = windows.get(window) else {
        return;
    };

    for event in events.iter() {
        for (key, rect) in &window.positions {
            if hit_test(*rect, cursor.position()) {
                let Some(handlers) = window.events.get(&key) else {
                    continue;
                };

                if let Some(f) = &handlers.mouse_button_input {
                    f(*event);
                }
            }
        }
    }
}

pub fn dispatch_received_character_events(
    cursor: Res<Cursor>,
    windows: Query<&Events>,
    mut events: EventReader<ReceivedCharacter>,
) {
    if events.is_empty() {
        return;
    }

    let Some(window) = cursor.window() else {
        return;
    };

    let Ok(window) = windows.get(window) else {
        return;
    };

    for event in events.iter() {
        for (key, rect) in &window.positions {
            if hit_test(*rect, cursor.position()) {
                let Some(handlers) = window.events.get(&key) else {
                    continue;
                };

                if let Some(f) = &handlers.received_character {
                    f(event.char);
                }
            }
        }
    }
}

pub fn hit_test(elem: Rect, cursor: Vec2) -> bool {
    cursor.x >= elem.min.x
        && cursor.x <= elem.max.x
        && cursor.y >= elem.min.y
        && cursor.y <= elem.max.y
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::hit_test;

    use super::Rect;

    #[test]
    fn hit_test_edge() {
        let elem = Rect {
            min: Vec2 { x: 0.0, y: 0.0 },
            max: Vec2 { x: 1.0, y: 1.0 },
        };
        let cursor = Vec2 { x: 0.0, y: 0.0 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_inside() {
        let elem = Rect {
            min: Vec2 { x: 0.0, y: 0.0 },
            max: Vec2 { x: 1.0, y: 1.0 },
        };
        let cursor = Vec2 { x: 0.5, y: 0.8 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_outside() {
        let elem = Rect {
            min: Vec2 { x: 0.0, y: 0.0 },
            max: Vec2 { x: 1.0, y: 1.0 },
        };
        let cursor = Vec2 { x: 1.1, y: 0.5 };

        assert!(!hit_test(elem, cursor));
    }
}
