use std::ops::Deref;

use winit::event::VirtualKeyCode;

use crate::events::{ElementEventHandlers, EventHandlers};
use crate::reactive::{create_effect, create_signal, Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::{Component, Text, TextProps};

pub struct InputProps {
    pub value: String,
    pub style: Style,
    pub on_change: Box<dyn Fn(String) + Send + Sync + 'static>,
}

pub struct Input;

impl Component for Input {
    type Properties = InputProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        let (value, set_value) = create_signal(cx, Buffer::new(props.value));

        let root = cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: props.style,
            },
            events: ElementEventHandlers {
                local: EventHandlers {
                    keyboard_input: Some(Box::new({
                        let set_value = set_value.clone();

                        move |event| {
                            if !event.state.is_pressed() {
                                return;
                            }

                            match event.key_code {
                                Some(VirtualKeyCode::Left) => {
                                    set_value.update(|string| string.move_back());
                                }
                                Some(VirtualKeyCode::Right) => {
                                    set_value.update(|string| string.move_forward());
                                }
                                Some(VirtualKeyCode::Home) => {
                                    set_value.update(|string| string.move_to_start());
                                }
                                Some(VirtualKeyCode::End) => {
                                    set_value.update(|string| string.move_to_end());
                                }
                                _ => (),
                            }
                        }
                    })),
                    received_character: Some(Box::new(move |char| match char {
                        // Return creates a newline.
                        '\r' => {
                            set_value.update(|string| {
                                string.push('\n');
                            });
                        }
                        // Backspace
                        '\u{8}' => set_value.update(|string| {
                            string.remove_prev();
                        }),
                        // Delete
                        '\u{7F}' => set_value.update(|string| {
                            string.remove_next();
                        }),
                        _ => {
                            if !char.is_control() {
                                set_value.update(|string| string.push(char));
                            }
                        }
                    })),
                    ..Default::default()
                },
                ..Default::default()
            },
        });

        {
            let value = value.clone();
            create_effect(cx, move |_| {
                let buffer = value.get();

                (props.on_change)(buffer.string);
            });
        }

        Text::render(
            &root,
            TextProps {
                text: (move || {
                    let buffer = value.get();

                    let mut string = buffer.to_string();
                    string.insert(buffer.cursor, '|');
                    string
                })
                .into(),
            },
        );

        root
    }
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
            let ch = s.chars().nth(0).unwrap();

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
