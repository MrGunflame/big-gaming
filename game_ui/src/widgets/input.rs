use std::collections::HashMap;
use std::ops::{Deref, Range};
use std::time::Duration;

use async_io::Timer;
use futures::StreamExt;
use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;
use glam::UVec2;
use image::Rgba;
use parking_lot::Mutex;

use crate::runtime::events::NodeDestroyed;
use crate::runtime::{ClipboardRef, Context, NodeId, Runtime, TaskHandle};
use crate::style::{Background, Bounds, Color, Size, SizeVec2, Style};

use super::{Callback, Container, Text, Widget};

// FIXME: Some platforms (e.g. Windows) have customizable blinking intervals
// that we should conform to (e.g. GetCaretBlinkTime for Windows).
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

const SELECTION_COLOR: Color = Color(Rgba([0x1e, 0x90, 0xff, 0xff]));

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
                background: Background::from_hex("42414d").unwrap(),
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
    fn mount(self, parent: &Context) -> Context {
        let _span = trace_span!("Input::mount").entered();

        let wrapper = Container::new().style(self.style).mount(parent);
        let text_node = Text::new(self.value.clone())
            .size(32.0)
            .mount(&wrapper)
            .node()
            .unwrap();

        parent
            .document()
            .register_with_parent(wrapper.node().unwrap(), {
                let ctx = wrapper.clone();
                move |event: NodeDestroyed| {
                    let node = ctx.node().unwrap();
                    let state = ctx.document().get::<InputState>().unwrap();
                    let mut active = state.active.lock();
                    let mut nodes = state.nodes.lock();

                    if active.as_ref().is_some_and(|active| active.node == node) {
                        *active = None;
                    }

                    nodes.remove(&node);
                }
            });

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
            let handle = parent.spawn_task(async move {
                let mut timer = Timer::interval(CARET_BLINK_INTERVAL);
                let mut cursor_blink = false;
                loop {
                    timer.next().await;

                    let Some(state) = ctx.document().get::<InputState>() else {
                        break;
                    };

                    let mut nodes = state.nodes.lock();
                    let active = state.active.lock();

                    let Some(active) = &*active else {
                        continue;
                    };

                    let node = nodes.get_mut(&active.node).unwrap();
                    node.ctx.clear_children();
                    let text = Text::new(node.buffer.string.clone())
                        .size(32.0)
                        .caret(cursor_blink.then_some(node.buffer.cursor as u32))
                        .selection_range(active.selection.map(|s| s.range()))
                        .selection_color(SELECTION_COLOR)
                        .mount(&node.ctx);
                    node.text_node = text.node().unwrap();
                    cursor_blink ^= true;
                }
            });

            let ctx = wrapper.clone();
            parent.document().register(move |event: MouseButtonInput| {
                let Some(state) = ctx.document().get::<InputState>() else {
                    return;
                };

                let mut nodes = state.nodes.lock();
                let mut active = state.active.lock();

                let prev_active = active.clone();

                let mut selected = false;
                for node in nodes.values() {
                    let Some(layout) = ctx.layout(node.ctx.node().unwrap()) else {
                        continue;
                    };

                    if layout.contains(ctx.cursor().position().unwrap_or_default()) {
                        *active = Some(ActiveNode {
                            node: node.ctx.node().unwrap(),
                            selection: None,
                        });
                        selected = true;
                        break;
                    }
                }

                if !selected {
                    *active = None
                }

                if let Some(prev_active) = prev_active {
                    let node = nodes.get_mut(&prev_active.node).unwrap();
                    node.ctx.clear_children();
                    let text = Text::new(node.buffer.string.clone())
                        .size(32.0)
                        .caret(None)
                        .mount(&node.ctx);
                    node.text_node = text.node().unwrap();
                }

                if let Some(active) = active.clone() {
                    let node = nodes.get_mut(&active.node).unwrap();

                    // Note that we must use the text node instead of the wrapper
                    // container node as the container may have additional styling
                    // (e.g. padding) that may change the layout and cause the position
                    // to become inprecise.
                    // The text node has no additional styling properties that may cause
                    // the layout to shift.
                    let layout = ctx.layout(node.text_node).unwrap();

                    // We detect whether an input is active based on the outer container.
                    // This statement therefore underflow if the user clicked on the
                    // padding area.
                    // FIXME: We treat that case as zero, but is this desired?
                    let position = ctx
                        .cursor()
                        .position()
                        .unwrap_or_default()
                        .saturating_sub(layout.min);

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
                key_states: Mutex::new(KeyStates {
                    lshift: false,
                    rshift: false,
                    lctrl: false,
                    rctrl: false,
                }),
            });

            let ctx = wrapper.clone();
            parent.document().register(move |event: KeyboardInput| {
                let Some(state) = ctx.document().get::<InputState>() else {
                    return;
                };

                match event.key_code {
                    Some(KeyCode::LShift) => {
                        state.key_states.lock().lshift = event.state.is_pressed();
                        return;
                    }
                    Some(KeyCode::RShift) => {
                        state.key_states.lock().rshift = event.state.is_pressed();
                        return;
                    }
                    Some(KeyCode::LControl) => {
                        state.key_states.lock().lctrl = event.state.is_pressed();
                        return;
                    }
                    Some(KeyCode::RControl) => {
                        state.key_states.lock().rctrl = event.state.is_pressed();
                        return;
                    }
                    _ => (),
                }

                let mut nodes = state.nodes.lock();
                let mut active = state.active.lock();

                let Some(active_node) = &mut *active else {
                    return;
                };

                let node = nodes.get_mut(&active_node.node).unwrap();
                let key_states = state.key_states.lock();
                if !update_buffer(
                    &ctx.clipboard(),
                    &mut node.buffer,
                    &key_states,
                    &mut active_node.selection,
                    &event,
                ) {
                    return;
                }

                node.ctx.clear_children();
                let text = Text::new(node.buffer.string.clone())
                    .size(32.0)
                    .caret(Some(node.buffer.cursor as u32))
                    .selection_range(active_node.selection.map(|s| s.range()))
                    .selection_color(SELECTION_COLOR)
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

fn update_buffer(
    clipboard: &ClipboardRef<'_>,
    buffer: &mut Buffer,
    key_states: &KeyStates,
    selection: &mut Option<Selection>,
    event: &KeyboardInput,
) -> bool {
    // Don't trigger when releasing the button.
    if !event.state.is_pressed() {
        return false;
    }

    match event.key_code {
        Some(KeyCode::C) if key_states.is_control_pressed() => {
            if let Some(selection) = selection {
                if let Some(text) = buffer.string.get(selection.range()) {
                    clipboard.set(text);
                }
            }

            return false;
        }
        Some(KeyCode::V) if key_states.is_control_pressed() => {
            if let Some(text) = clipboard.get() {
                buffer.insert(&text);
            }

            return true;
        }
        _ => (),
    }

    let cursor_start = buffer.cursor;

    let is_move_op = match event.key_code {
        Some(KeyCode::Left) => {
            if key_states.is_control_pressed() {
                buffer.move_back_word();
            } else {
                buffer.move_back();
            }
            true
        }
        Some(KeyCode::Right) => {
            if key_states.is_control_pressed() {
                buffer.move_forward_word();
            } else {
                buffer.move_forward();
            }
            true
        }
        Some(KeyCode::Home) => {
            buffer.move_to_start();
            true
        }
        Some(KeyCode::End) => {
            buffer.move_to_end();
            true
        }
        _ => false,
    };

    if is_move_op {
        if key_states.is_shift_pressed() {
            match selection {
                Some(selection) => selection.end = buffer.cursor,
                None => {
                    *selection = Some(Selection {
                        start: cursor_start,
                        end: buffer.cursor,
                    })
                }
            }
        } else {
            *selection = None;
        }

        return true;
    }

    let is_edited = match event.text.as_ref().map(|s| s.as_str()) {
        Some("\r") => {
            buffer.push('\n');
            true
        }
        // Backspace
        Some("\u{8}") => {
            if let Some(selection) = selection {
                buffer.remove_range(selection.range());
            } else {
                buffer.remove_prev();
            }

            true
        }
        // Delete
        Some("\u{7F}") => {
            if let Some(selection) = selection {
                buffer.remove_range(selection.range());
            } else {
                buffer.remove_next();
            }

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
    };

    *selection = None;
    is_edited
}

#[derive(Debug)]
struct InputState {
    nodes: Mutex<HashMap<NodeId, NodeState>>,
    active: Mutex<Option<ActiveNode>>,
    /// The handle to the task handling periodic caret blinking.
    ///
    /// We keep a handle to the task so that it gets dropped together with all other state when the
    /// last `Input` element in a document gets destroyed.
    _caret_blink_task: TaskHandle<()>,
    key_states: Mutex<KeyStates>,
}

#[derive(Clone, Debug)]
struct ActiveNode {
    node: NodeId,
    selection: Option<Selection>,
}

#[derive(Copy, Clone, Debug)]
struct Selection {
    start: usize,
    end: usize,
}

impl Selection {
    fn range(&self) -> Range<usize> {
        let start = usize::min(self.start, self.end);
        let end = usize::max(self.start, self.end);
        start..end
    }
}

#[derive(Copy, Clone, Debug)]
struct KeyStates {
    lshift: bool,
    rshift: bool,
    lctrl: bool,
    rctrl: bool,
}

impl KeyStates {
    /// Returns `true` if any `Shift` key is pressed.
    fn is_shift_pressed(&self) -> bool {
        self.lshift || self.rshift
    }

    /// Returns `true` if any `Control` key is pressed.
    fn is_control_pressed(&self) -> bool {
        self.lctrl || self.rctrl
    }
}

#[derive(Debug)]
struct NodeState {
    ctx: Context,
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

    fn insert(&mut self, string: &str) {
        self.string.insert_str(self.cursor, string);
        self.cursor += string.len();
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

    fn remove_range(&mut self, range: Range<usize>) {
        if range.end > self.string.len() {
            return;
        }

        if self.cursor > range.start {
            self.cursor -= range.len();
        }

        self.string.replace_range(range, "");
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

    fn move_forward_word(&mut self) {
        if let Some(s) = self.string.get(self.cursor..) {
            let mut leading_punctuation = true;
            for ch in s.chars() {
                if ch.is_ascii_punctuation() || ch.is_whitespace() {
                    if !leading_punctuation {
                        break;
                    }
                } else {
                    leading_punctuation = false;
                }

                self.cursor += ch.len_utf8();
            }
        }
    }

    fn move_back_word(&mut self) {
        if let Some(s) = self.string.get(..self.cursor) {
            let mut trailing_punctuation = true;
            for ch in s.chars().rev() {
                if ch.is_ascii_punctuation() || ch.is_whitespace() {
                    if !trailing_punctuation {
                        break;
                    }
                } else {
                    trailing_punctuation = false;
                }

                self.cursor -= ch.len_utf8();
            }
        }
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

    #[test]
    fn buffer_move_forward_word() {
        let string = String::from("Hello World");

        let mut buffer = Buffer::new(string);
        buffer.cursor = 0;
        buffer.move_forward_word();

        assert_eq!(buffer.cursor, 5);
    }

    #[test]
    fn buffer_move_forward_leading_whitespace() {
        let string = String::from("  Hello World");

        let mut buffer = Buffer::new(string);
        buffer.cursor = 0;
        buffer.move_forward_word();

        assert_eq!(buffer.cursor, 7);
    }

    #[test]
    fn buffer_move_back_word() {
        let string = String::from("Hello World");

        let mut buffer = Buffer::new(string);
        buffer.move_back_word();

        assert_eq!(buffer.cursor, 6);
    }

    #[test]
    fn buffer_move_back_trailing_whitespace() {
        let string = String::from("Hello World  ");

        let mut buffer = Buffer::new(string);
        buffer.move_back_word();

        assert_eq!(buffer.cursor, 6);
    }

    #[test]
    fn buffer_remove_range() {
        let string = String::from("Hello World");

        let mut buffer = Buffer::new(string);
        buffer.remove_range(1..7);

        assert_eq!(buffer.string, "Horld");
        assert_eq!(buffer.cursor, 5);
    }
}
