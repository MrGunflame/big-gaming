use crate::events::EventHandlers;
use crate::reactive::{create_signal, Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::{Component, Text, TextProps};

pub struct InputProps {
    pub value: String,
    pub style: Style,
}

pub struct Input;

impl Component for Input {
    type Properties = InputProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        let (value, set_value) = create_signal(cx, props.value);

        let root = cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: props.style,
            },
            events: EventHandlers {
                received_character: Some(Box::new(move |char| match char {
                    // Return creates a newline.
                    '\r' => {
                        set_value.update(|string| {
                            string.push('\n');
                        });
                    }
                    // Backspace
                    '\u{8}' => set_value.update(|string| {
                        if !string.is_empty() {
                            string.remove(string.len() - 1);
                        }
                    }),
                    // Delete
                    '\u{7F}' => {}
                    _ => {
                        if !char.is_control() {
                            set_value.update(|string| string.push(char));
                        }
                    }
                })),
                ..Default::default()
            },
        });

        Text::render(
            &root,
            TextProps {
                text: (move || value.get()).into(),
            },
        );

        root
    }
}
