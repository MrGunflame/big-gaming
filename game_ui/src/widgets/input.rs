use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use game_input::keyboard::{KeyCode, KeyboardInput};
use game_input::mouse::MouseButtonInput;

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::{Bounds, Size, SizeVec2, Style};

use super::{Callback, Container, Text, Widget};

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
        let mut buffer = Buffer::new(self.value.clone());
        let is_selected = Arc::new(AtomicBool::new(false));

        let node_ctx = Container::new().style(self.style).mount(parent);
        let node_id = node_ctx.node().unwrap();

        {
            let node_ctx = node_ctx.clone();
            let is_selected = is_selected.clone();
            parent
                .document()
                .register(move |ctx: Context<KeyboardInput>| {
                    if !is_selected.load(Ordering::Acquire)
                        || !update_buffer(&mut buffer, &ctx.event)
                    {
                        return;
                    }

                    node_ctx.clear_children();
                    let text = Text::new(buffer.string.clone()).size(32.0);
                    text.mount(&node_ctx);

                    self.on_change.call(buffer.string.clone());
                });
        }

        parent
            .document()
            .register(move |ctx: Context<MouseButtonInput>| {
                if let Some(node) = ctx.layout(node_id) {
                    if node.contains(ctx.cursor().as_uvec2()) {
                        is_selected.store(true, Ordering::Release);
                    } else {
                        is_selected.store(false, Ordering::Release);
                    }
                }
            });

        let text = Text::new(self.value).size(32.0);
        text.clone().mount(&node_ctx);

        node_ctx
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

/// A UTF-8 string buffer.
#[derive(Clone, Debug)]
struct Buffer {
    string: String,
    /// Position of the cursor.
    cursor: usize,
    /// Whether the buffer was update by the user.
    ///
    /// This is important to break circular updates when the value changes via a read signal.
    user_updated: bool,
}

impl Buffer {
    fn new(string: String) -> Self {
        let cursor = string
            .char_indices()
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        Self {
            string,
            cursor,
            user_updated: false,
        }
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
