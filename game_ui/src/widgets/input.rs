use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;

use async_io::Timer;
use futures::StreamExt;
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;
use glam::UVec2;
use parking_lot::Mutex;

use crate::reactive::{Context, NodeDestroyed, NodeId, TaskHandle};
use crate::style::{Bounds, Size, SizeVec2, Style};

use super::{Callback, Container, Text, Widget};

// FIXME: Some platforms (e.g. Windows) have customizable blinking intervals
// that we should conform to (e.g. GetCaretBlinkTime for Windows).
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

pub struct Input {
    pub value: String,
    pub on_change: Callback<String>,
    pub style: Style,
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            on_change: Callback::default(),
            style: Style {
                // Minimum size to prevent the input widget to
                // completely disappear.
                bounds: Bounds::from_min(SizeVec2::splat(Size::Pixels(10))),
                ..Default::default()
            },
        }
    }

    pub fn value<T>(mut self, value: T) -> Self
    where
        T: ToString,
    {
        self.value = value.to_string();
        self
    }

    pub fn on_change<T>(mut self, on_change: T) -> Self
    where
        T: Into<Callback<String>>,
    {
        self.on_change = on_change.into();
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Input {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let _span = trace_span!("Input::mount").entered();

        let wrapper = Container::new().style(self.style).mount(parent);
        let text_node = Text::new(self.value.clone())
            .size(32.0)
            .mount(&wrapper)
            .node()
            .unwrap();

        parent.document().register_with_parent(
            wrapper.node().unwrap(),
            move |ctx: Context<NodeDestroyed>| {
                let node = ctx.node().unwrap();
                let state = ctx.document().get::<InputState>().unwrap();
                let mut active = state.active.lock();
                let mut nodes = state.nodes.lock();

                if *active == Some(node) {
                    *active = None;
                }

                nodes.remove(&node);
                // if nodes.is_empty() {
                //     ctx.document().remove::<InputState>();
                // }
            },
        );

        if let Some(state) = parent.document().get::<InputState>() {
            tracing::debug!("reusing existing InputState");

            state.nodes.lock().insert(
                wrapper.node().unwrap(),
                NodeState {
                    ctx: wrapper.clone(),
                    on_change: self.on_change,
                    buffer: Buffer::new(self.value),
                    text_node,
                },
            );
        } else {
            tracing::debug!("creating new InputState");

            let mut nodes = HashMap::new();
            nodes.insert(
                wrapper.node().unwrap(),
                NodeState {
                    ctx: wrapper.clone(),
                    on_change: self.on_change,
                    buffer: Buffer::new(self.value),
                    text_node,
                },
            );

            let ctx = wrapper.clone();
            let handle = parent.runtime().spawn_task(async move {
                let mut timer = Timer::interval(CARET_BLINK_INTERVAL);
                let mut cursor_blink = false;
                loop {
                    timer.next().await;

                    let Some(state) = ctx.document().get::<InputState>() else {
                        break;
                    };

                    let mut nodes = state.nodes.lock();
                    let active = state.active.lock();

                    let Some(node) = *active else {
                        continue;
                    };

                    let node = nodes.get_mut(&node).unwrap();
                    node.ctx.clear_children();
                    let text = Text::new(node.buffer.string.clone())
                        .size(32.0)
                        .caret(cursor_blink.then_some(node.buffer.cursor as u32))
                        .mount(&node.ctx);
                    node.text_node = text.node().unwrap();
                    cursor_blink ^= true;
                }
            });

            parent
                .document()
                .register(move |ctx: Context<MouseButtonInput>| {
                    let Some(state) = ctx.document().get::<InputState>() else {
                        return;
                    };

                    let mut nodes = state.nodes.lock();
                    let mut active = state.active.lock();

                    let prev_active = *active;

                    let mut selected = false;
                    for node in nodes.values() {
                        let Some(layout) = ctx.layout(node.ctx.node().unwrap()) else {
                            continue;
                        };

                        if layout.contains(ctx.cursor().as_uvec2()) {
                            *active = Some(node.ctx.node().unwrap());
                            selected = true;
                            break;
                        }
                    }

                    if !selected {
                        *active = None
                    }

                    if let Some(prev_active) = prev_active {
                        let node = nodes.get_mut(&prev_active).unwrap();
                        node.ctx.clear_children();
                        let text = Text::new(node.buffer.string.clone())
                            .size(32.0)
                            .caret(None)
                            .mount(&node.ctx);
                        node.text_node = text.node().unwrap();
                    }

                    if let Some(active) = *active {
                        let node = nodes.get_mut(&active).unwrap();

                        // Note that we must use the text node instead of the wrapper
                        // container node as the container may have additional styling
                        // (e.g. padding) that may change the layout and cause the position
                        // to become inprecise.
                        // The text node has no additional styling properties that may cause
                        // the layout to shift.
                        let layout = ctx.layout(node.text_node).unwrap();
                        let position = ctx.cursor().as_uvec2() - layout.min;

                        let cursor = crate::render::text::get_position_in_text(
                            &node.buffer,
                            32.0,
                            UVec2::MAX,
                            position,
                        );
                        node.buffer.cursor = cursor;

                        node.ctx.clear_children();

                        let text = Text::new(node.buffer.string.clone())
                            .size(32.0)
                            .caret(Some(node.buffer.cursor as u32))
                            .mount(&node.ctx);
                        node.text_node = text.node().unwrap();
                    }
                });

            parent.document().insert(InputState {
                nodes: Mutex::new(nodes),
                active: Mutex::new(None),
                _caret_blink_task: handle,
            });

            parent
                .document()
                .register(move |ctx: Context<KeyboardInput>| {
                    let Some(state) = ctx.document().get::<InputState>() else {
                        return;
                    };

                    let mut nodes = state.nodes.lock();
                    let active = state.active.lock();

                    let Some(node) = &*active else {
                        return;
                    };

                    let node = nodes.get_mut(node).unwrap();
                    if !update_buffer(&mut node.buffer, &ctx.event) {
                        return;
                    }

                    node.ctx.clear_children();
                    let text = Text::new(node.buffer.string.clone())
                        .size(32.0)
                        .caret(Some(node.buffer.cursor as u32))
                        .mount(&node.ctx);
                    node.text_node = text.node().unwrap();

                    let string = node.buffer.string.clone();
                    let on_change = node.on_change.clone();
                    drop(nodes);
                    drop(active);
                    on_change.call(string);
                });
        }

        wrapper
    }
}

fn update_buffer(buffer: &mut Buffer, event: &KeyboardInput) -> bool {
    // Don't trigger when releasing the button.
    if !event.state.is_pressed() {
        return false;
    }

    match event.key_code {
        Some(KeyCode::Left) => {
            buffer.move_back();
            return true;
        }
        Some(KeyCode::Right) => {
            buffer.move_forward();
            return true;
        }
        Some(KeyCode::Home) => {
            buffer.move_to_start();
            return true;
        }
        Some(KeyCode::End) => {
            buffer.move_to_end();
            return true;
        }
        _ => (),
    }

    match event.text.as_ref().map(|s| s.as_str()) {
        Some("\r") => {
            buffer.push('\n');
            true
        }
        // Backspace
        Some("\u{8}") => {
            buffer.remove_prev();
            true
        }
        // Delete
        Some("\u{7F}") => {
            buffer.remove_next();
            true
        }
        Some(text) => {
            for char in text.chars() {
                if !char.is_control() {
                    buffer.push(char);
                }
            }

            true
        }
        _ => false,
    }
}

#[derive(Debug)]
struct InputState {
    nodes: Mutex<HashMap<NodeId, NodeState>>,
    active: Mutex<Option<NodeId>>,
    /// The handle to the task handling periodic caret blinking.
    ///
    /// We keep a handle to the task so that it gets dropped together with all other state when the
    /// last `Input` element in a document gets destroyed.
    _caret_blink_task: TaskHandle<()>,
}

#[derive(Debug)]
struct NodeState {
    ctx: Context<()>,
    text_node: NodeId,
    on_change: Callback<String>,
    buffer: Buffer,
}

/// A UTF-8 string buffer.
#[derive(Clone, Debug)]
struct Buffer {
    string: String,
    /// Position of the cursor.
    cursor: usize,
}

impl Buffer {
    fn new(string: String) -> Self {
        let cursor = string
            .char_indices()
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        Self { string, cursor }
    }

    fn push(&mut self, ch: char) {
        self.string.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    fn remove_next(&mut self) {
        if let Some(s) = self.string.get(self.cursor..) {
            if s.is_empty() {
                return;
            }

            // self.cursor exists means that at least 1 char exists.
            if cfg!(debug_assertions) {
                s.chars().nth(0).unwrap();
            }

            self.string.remove(self.cursor);
        }
    }

    fn remove_prev(&mut self) {
        let s = &self.string[..self.cursor];

        if let Some(ch) = s.chars().last() {
            self.string.remove(self.cursor - ch.len_utf8());
            self.cursor -= ch.len_utf8();
        }
    }

    fn move_forward(&mut self) {
        if let Some(s) = self.string.get(self.cursor..) {
            if let Some(ch) = s.chars().next() {
                self.cursor += ch.len_utf8();
            }
        }
    }

    fn move_back(&mut self) {
        if let Some(s) = self.string.get(..self.cursor) {
            if let Some(ch) = s.chars().last() {
                self.cursor -= ch.len_utf8();
            }
        }
    }

    fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    fn move_to_end(&mut self) {
        self.cursor = self.string.len();
    }
}

impl Deref for Buffer {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

#[cfg(test)]
mod tests {
    use super::Buffer;

    #[test]
    fn buffer_new_empty() {
        let string = String::new();

        let buffer = Buffer::new(string.clone());

        assert_eq!(buffer.string, string);
        assert_eq!(buffer.cursor, 0);
    }

    #[test]
    fn buffer_from_ascii() {
        let string = String::from("test");

        let buffer = Buffer::new(string.clone());

        assert_eq!(buffer.string, string);
        assert_eq!(buffer.cursor, 4);
    }

    #[test]
    fn buffer_from_unicode() {
        let string = String::from("testö");

        let buffer = Buffer::new(string.clone());

        assert_eq!(buffer.string, string);
        assert_eq!(buffer.cursor, 6);
    }

    #[test]
    fn buffer_remove_next() {
        let string = String::from("testö");

        let mut buffer = Buffer::new(string.clone());
        buffer.remove_next();

        assert_eq!(buffer.string, string);
        assert_eq!(buffer.cursor, 6);
    }

    #[test]
    fn buffer_remove_prev() {
        let string = String::from("testö");

        let mut buffer = Buffer::new(string.clone());
        buffer.remove_prev();

        assert_eq!(buffer.string, "test");
        assert_eq!(buffer.cursor, 4);
    }

    #[test]
    fn buffer_move_forward_ok() {
        let string = String::from("öäü");

        let mut buffer = Buffer::new(string);
        buffer.cursor = 0;
        buffer.move_forward();

        assert_eq!(buffer.cursor, 2);
    }

    #[test]
    fn buffer_move_forward_eol() {
        let string = String::from("öäü");

        let mut buffer = Buffer::new(string);
        buffer.move_forward();

        assert_eq!(buffer.cursor, 6);
    }

    #[test]
    fn buffer_move_back_ok() {
        let string = String::from("öäü");

        let mut buffer = Buffer::new(string);
        buffer.move_back();

        assert_eq!(buffer.cursor, 4);
    }

    #[test]
    fn buffer_move_back_eol() {
        let string = String::from("öäü");

        let mut buffer = Buffer::new(string);
        buffer.cursor = 0;
        buffer.move_back();

        assert_eq!(buffer.cursor, 0);
    }
}
