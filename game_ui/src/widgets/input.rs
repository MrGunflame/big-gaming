use std::ops::Deref;

use game_input::keyboard::{KeyCode, KeyboardInput};

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::Style;

use super::{Text, Widget};

pub struct Input {
    pub value: String,
    pub on_change: (),
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            on_change: (),
        }
    }

    pub fn value<T>(mut self, value: T) -> Self
    where
        T: ToString,
    {
        self.value = value.to_string();
        self
    }
}

impl Widget for Input {
    fn mount<T>(self, parent: &Context<T>) {
        let mut node = Node::new(Primitive {
            style: Style::default(),
            image: None,
            text: None,
        });

        let mut buffer = Buffer::new(self.value.clone());

        node.register(move |ctx: Context<KeyboardInput>| {
            if !update_buffer(&mut buffer, &ctx.event) {
                return;
            }

            dbg!(&ctx.event);

            ctx.clear_children();
            let text = Text::new(buffer.string.clone()).size(32.0);
            text.mount(&ctx);
        });

        let ctx = parent.append(node);

        let text = Text::new(self.value).size(32.0);
        text.clone().mount(&ctx);
        // text.mount(&parent);
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
