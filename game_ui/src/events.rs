use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::ptr::NonNull;
use std::sync::{mpsc, Arc};

use game_input::keyboard::KeyboardInput;
use game_input::mouse::{MouseButtonInput, MouseWheel};
use game_window::cursor::{Cursor, CursorIcon};
use game_window::events::{CursorMoved, ReceivedCharacter};
use game_window::windows::WindowId;
use glam::{UVec2, Vec2};

use crate::layout::{Key, LayoutTree};
use crate::render::Rect;

#[derive(Clone, Debug)]
pub struct Context<T> {
    pub cursor: Arc<Cursor>,
    pub event: T,
    pub window: WindowContext,
    _priv: (),
}

impl<T> Context<T> {
    fn with_event<U>(self, event: U) -> Context<U> {
        Context {
            cursor: self.cursor,
            event,
            window: self.window,
            _priv: (),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WindowContext {
    window: WindowId,
    tx: mpsc::Sender<WindowCommand>,
}

impl WindowContext {
    pub fn close(&self) {
        let _ = self.tx.send(WindowCommand::Close(self.window));
    }

    pub fn set_title<T>(&self, title: T)
    where
        T: ToString,
    {
        let _ = self
            .tx
            .send(WindowCommand::SetTitle(self.window, title.to_string()));
    }

    pub fn set_cursor_icon(&self, icon: CursorIcon) {
        let _ = self
            .tx
            .send(WindowCommand::SetCursorIcon(self.window, icon));
    }
}

#[derive(Clone, Debug)]
pub(crate) enum WindowCommand {
    Close(WindowId),
    SetTitle(WindowId, String),
    SetCursorIcon(WindowId, CursorIcon),
}

#[derive(Debug, Default)]
pub struct ElementEventHandlers {
    pub local: EventHandlers,
    pub global: EventHandlers,
}

#[derive(Default)]
pub struct EventHandlers {
    pub cursor_moved: Option<Box<dyn Fn(Context<CursorMoved>) + Send + Sync + 'static>>,
    pub cursor_left: Option<Box<dyn Fn(Context<()>) + Send + Sync + 'static>>,
    pub cursor_entered: Option<Box<dyn Fn(Context<()>) + Send + Sync + 'static>>,
    pub mouse_button_input: Option<Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>>,
    pub mouse_wheel: Option<Box<dyn Fn(Context<MouseWheel>) + Send + Sync + 'static>>,
    pub keyboard_input: Option<Box<dyn Fn(Context<KeyboardInput>) + Send + Sync + 'static>>,
    pub received_character: Option<Box<dyn Fn(Context<ReceivedCharacter>) + Send + Sync + 'static>>,
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
            .field("mouse_wheel", &map_to_ptr(&self.mouse_wheel))
            .field("keyboard_input", &map_to_ptr(&self.keyboard_input))
            .field("received_character", &map_to_ptr(&self.received_character))
            .finish()
    }
}

#[derive(Default)]
pub struct Events {
    events: HashMap<Key, ElementEventHandlers>,
    positions: Vec<(Key, Rect)>,
    hovered_elements: HashSet<Key>,
}

impl Events {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            positions: Vec::new(),
            hovered_elements: HashSet::new(),
        }
    }

    pub fn insert(&mut self, key: Key, handlers: ElementEventHandlers) {
        self.events.insert(key, handlers);
    }

    pub fn remove(&mut self, key: Key) {
        self.events.remove(&key);
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut ElementEventHandlers> {
        self.events.get_mut(&key)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub fn update_events_from_layout_tree(tree: &mut LayoutTree, events: &mut Events) {
    events.positions.clear();

    for (key, layout) in tree.keys().zip(tree.layouts()) {
        let position = Rect {
            min: layout.position,
            max: UVec2::new(
                layout.position.x + layout.width,
                layout.position.y + layout.height,
            ),
        };

        events.positions.push((key, position));
    }
}

pub(crate) fn dispatch_cursor_moved_events(
    tx: &mpsc::Sender<WindowCommand>,
    cursor: &Arc<Cursor>,
    windows: &mut HashMap<WindowId, Events>,
    event: CursorMoved,
) {
    let Some(window) = windows.get_mut(&event.window) else {
        return;
    };

    let mut hovered = window.hovered_elements.clone();
    window.hovered_elements.clear();

    for (key, rect) in &window.positions {
        let Some(handlers) = window.events.get(&key) else {
            continue;
        };

        let ctx = Context {
            cursor: cursor.clone(),
            event,
            window: WindowContext {
                window: event.window,
                tx: tx.clone(),
            },
            _priv: (),
        };

        if let Some(f) = &handlers.global.cursor_moved {
            f(ctx.clone());
        }

        if hit_test(*rect, event.position) {
            if let Some(f) = &handlers.local.cursor_moved {
                f(ctx.clone());
            }

            window.hovered_elements.insert(*key);
            if !hovered.remove(key) {
                if let Some(f) = &handlers.local.cursor_entered {
                    f(ctx.clone().with_event(()));
                }
            }
        }
    }

    for key in hovered {
        let Some(handlers) = window.events.get(&key) else {
            continue;
        };

        let ctx = Context {
            cursor: cursor.clone(),
            event: (),
            window: WindowContext {
                window: event.window,
                tx: tx.clone(),
            },
            _priv: (),
        };

        if let Some(f) = &handlers.local.cursor_left {
            f(ctx.clone());
        }
    }
}

pub(crate) fn dispatch_mouse_button_input_events(
    tx: &mpsc::Sender<WindowCommand>,
    cursor: &Arc<Cursor>,
    windows: &HashMap<WindowId, Events>,
    event: MouseButtonInput,
) {
    let Some(window) = cursor.window() else {
        return;
    };

    let Some(window) = windows.get(&window) else {
        return;
    };

    for (key, rect) in &window.positions {
        let Some(handlers) = window.events.get(&key) else {
            continue;
        };

        let ctx = Context {
            cursor: cursor.clone(),
            event,
            window: WindowContext {
                window: cursor.window().unwrap(),
                tx: tx.clone(),
            },
            _priv: (),
        };

        if let Some(f) = &handlers.global.mouse_button_input {
            f(ctx.clone());
        }

        if hit_test(*rect, cursor.position()) {
            if let Some(f) = &handlers.local.mouse_button_input {
                f(ctx);
            }
        }
    }
}

pub(crate) fn dispatch_mouse_wheel_events(
    tx: &mpsc::Sender<WindowCommand>,
    cursor: &Arc<Cursor>,
    windows: &HashMap<WindowId, Events>,
    event: MouseWheel,
) {
    let Some(window) = cursor.window() else {
        return;
    };

    let Some(window) = windows.get(&window) else {
        return;
    };

    for (key, rect) in &window.positions {
        let Some(handlers) = window.events.get(&key) else {
            continue;
        };

        let ctx = Context {
            cursor: cursor.clone(),
            event,
            window: WindowContext {
                window: cursor.window().unwrap(),
                tx: tx.clone(),
            },
            _priv: (),
        };

        if let Some(f) = &handlers.global.mouse_wheel {
            f(ctx.clone());
        }

        if hit_test(*rect, cursor.position()) {
            if let Some(f) = &handlers.local.mouse_wheel {
                f(ctx);
            }
        }
    }
}

pub(crate) fn dispatch_received_character_events(
    tx: &mpsc::Sender<WindowCommand>,
    cursor: &Arc<Cursor>,
    windows: &HashMap<WindowId, Events>,
    event: ReceivedCharacter,
) {
    let Some(window) = cursor.window() else {
        return;
    };

    let Some(window) = windows.get(&window) else {
        return;
    };

    for (key, rect) in &window.positions {
        let Some(handlers) = window.events.get(&key) else {
            continue;
        };

        let ctx = Context {
            cursor: cursor.clone(),
            event,
            window: WindowContext {
                window: cursor.window().unwrap(),
                tx: tx.clone(),
            },
            _priv: (),
        };

        if let Some(f) = &handlers.global.received_character {
            f(ctx.clone());
        }

        if hit_test(*rect, cursor.position()) {
            if let Some(f) = &handlers.local.received_character {
                f(ctx);
            }
        }
    }
}

pub(crate) fn dispatch_keyboard_input_events(
    tx: &mpsc::Sender<WindowCommand>,
    cursor: &Arc<Cursor>,
    windows: &HashMap<WindowId, Events>,
    event: KeyboardInput,
) {
    let Some(window) = cursor.window() else {
        return;
    };

    let Some(window) = windows.get(&window) else {
        return;
    };

    for (key, rect) in &window.positions {
        let Some(handlers) = window.events.get(&key) else {
            continue;
        };

        let ctx = Context {
            cursor: cursor.clone(),
            event,
            window: WindowContext {
                window: cursor.window().unwrap(),
                tx: tx.clone(),
            },
            _priv: (),
        };

        if let Some(f) = &handlers.global.keyboard_input {
            f(ctx.clone());
        }

        if hit_test(*rect, cursor.position()) {
            if let Some(f) = &handlers.local.keyboard_input {
                f(ctx);
            }
        }
    }
}

pub fn hit_test(elem: Rect, cursor: Vec2) -> bool {
    // FIXME: Should this maybe be int cmp?
    // Floats are only relevant if cursor really uses its
    // full float range.
    cursor.x >= elem.min.x as f32
        && cursor.x <= elem.max.x as f32
        && cursor.y >= elem.min.y as f32
        && cursor.y <= elem.max.y as f32
}

#[cfg(test)]
mod tests {
    use glam::UVec2;
    use glam::Vec2;

    use super::hit_test;

    use super::Rect;

    #[test]
    fn hit_test_edge() {
        let elem = Rect {
            min: UVec2 { x: 0, y: 0 },
            max: UVec2 { x: 1, y: 1 },
        };
        let cursor = Vec2 { x: 0.0, y: 0.0 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_inside() {
        let elem = Rect {
            min: UVec2 { x: 0, y: 0 },
            max: UVec2 { x: 1, y: 1 },
        };
        let cursor = Vec2 { x: 0.5, y: 0.8 };

        assert!(hit_test(elem, cursor));
    }

    #[test]
    fn hit_test_outside() {
        let elem = Rect {
            min: UVec2 { x: 0, y: 0 },
            max: UVec2 { x: 1, y: 1 },
        };
        let cursor = Vec2 { x: 1.1, y: 0.5 };

        assert!(!hit_test(elem, cursor));
    }
}
