use std::ops::Deref;

use game_input::keyboard::KeyCode;
use game_window::cursor::CursorIcon;

use crate::events::{ElementEventHandlers, EventHandlers};
use crate::reactive::{Node, Scope};
use crate::render::{Element, ElementBody};
use crate::style::Style;

use super::text::Text;
use super::{Callback, ValueProvider, Widget};

pub struct Input {
    value: ValueProvider<String>,
    style: Style,
    on_change: Option<Callback<String>>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: ValueProvider::Static(String::new()),
            style: Style::default(),
            on_change: None,
        }
    }

    pub fn value<T>(mut self, value: T) -> Self
    where
        T: Into<ValueProvider<String>>,
    {
        self.value = value.into();
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn on_change<T>(mut self, on_change: T) -> Self
    where
        T: Into<Callback<String>>,
    {
        self.on_change = Some(on_change.into());
        self
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Input {
    fn build(self, cx: &Scope) -> Scope {
        let (value, set_value) = cx.create_signal(self.value.get());
        let (buffer, set_buffer) = cx.create_signal(Buffer::new(self.value.get()));

        match self.value {
            ValueProvider::Static(_) => (),
            ValueProvider::Reader(reader) => {
                let set_buffer = set_buffer.clone();
                cx.create_effect(move || {
                    let value = reader.get();

                    set_value.update(|val| *val = value.clone());
                    set_buffer.update(|buf| {
                        buf.string = value;
                        // We don't know if the new string has the same
                        // length, so just reset the cursor to start.
                        buf.move_to_start();
                        buf.user_updated = false;
                    })
                });
            }
        }

        let (focus, set_focus) = cx.create_signal(false);

        let root = cx.push(Node {
            element: Element {
                body: ElementBody::Container,
                style: self.style,
            },
            events: ElementEventHandlers {
                global: EventHandlers {
                    keyboard_input: Some(Box::new({
                        let set_value = set_buffer.clone();
                        let focus = focus.clone();

                        move |ctx| {
                            if !focus.get_untracked() {
                                return;
                            }

                            if !ctx.event.state.is_pressed() {
                                return;
                            }

                            match ctx.event.key_code {
                                Some(KeyCode::Left) => {
                                    set_value.update(|string| string.move_back());
                                }
                                Some(KeyCode::Right) => {
                                    set_value.update(|string| string.move_forward());
                                }
                                Some(KeyCode::Home) => {
                                    set_value.update(|string| string.move_to_start());
                                }
                                Some(KeyCode::End) => {
                                    set_value.update(|string| string.move_to_end());
                                }
                                _ => (),
                            }

                            match ctx.event.text.as_ref().map(|s| s.as_str()) {
                                // Return creates a newline.
                                Some("\r") => {
                                    set_buffer.update(|string| {
                                        string.push('\n');
                                        string.user_updated = true;
                                    });
                                }
                                // Backspace
                                Some("\u{8}") => set_buffer.update(|string| {
                                    string.remove_prev();
                                    string.user_updated = true;
                                }),
                                // Delete
                                Some("\u{7F}") => set_buffer.update(|string| {
                                    string.remove_next();
                                    string.user_updated = true;
                                }),
                                Some(text) => {
                                    for char in text.chars() {
                                        if !char.is_control() {
                                            set_buffer.update(|string| {
                                                string.push(char);
                                                string.user_updated = true;
                                            });
                                        }
                                    }
                                }
                                None => (),
                            }
                        }
                    })),
                    mouse_button_input: Some(Box::new({
                        let set_focus = set_focus.clone();

                        // Whenever we receive a click we remove focus from the input
                        // element. If the cursor clicks the input element the local
                        // handlers catches this afterwards.
                        // FIXME: This is exploiting the fact that global handlers are
                        // called before local ones, which is currently unspecified.
                        move |_ctx| {
                            set_focus.set(false);
                        }
                    })),
                    ..Default::default()
                },
                local: EventHandlers {
                    cursor_entered: Some(Box::new(move |ctx| {
                        ctx.window.set_cursor_icon(CursorIcon::Text);
                    })),
                    cursor_left: Some(Box::new(move |ctx| {
                        ctx.window.set_cursor_icon(CursorIcon::Default);
                    })),
                    mouse_button_input: Some(Box::new(move |_ctx| {
                        set_focus.set(true);
                    })),
                    ..Default::default()
                },
            },
        });

        {
            let value = buffer.clone();
            cx.create_effect(move || {
                let buffer = value.get();

                // Only update if the user has caused the change. This is
                // important because we don't want to call `on_change` if
                // the value changed via a `ReadSignal`.
                if !buffer.user_updated {
                    return;
                }

                if let Some(cb) = &self.on_change {
                    (cb.0)(buffer.string);
                }
            });
        }

        root.append(Text::new().text(ValueProvider::Reader(value)));

        root
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
