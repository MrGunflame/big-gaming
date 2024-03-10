use crate::events::ElementEventHandlers;
use crate::reactive::{Node, Scope};
use crate::render::{Element, ElementBody};
use crate::style::Style;

use super::{Container, ValueProvider, Widget};

#[derive(Clone, Debug)]
pub struct Text {
    text: ValueProvider<String>,
}

impl Text {
    pub const fn new() -> Self {
        Self {
            text: ValueProvider::Static(String::new()),
        }
    }

    pub fn text<T>(mut self, text: T) -> Self
    where
        T: Into<ValueProvider<String>>,
    {
        self.text = text.into();
        self
    }
}

impl Widget for Text {
    fn build(self, cx: &Scope) -> Scope {
        match self.text {
            ValueProvider::Static(text) => cx.push(build_node(text)),
            ValueProvider::Reader(reader) => {
                let root = cx.append(Container::new());

                let mut id = None;
                let root2 = root.clone();
                cx.create_effect(move || {
                    let text = reader.get();

                    if let Some(id) = id {
                        root2.remove(id);
                    }

                    id = root2.push(build_node(text)).id();
                });

                root
            }
        }
    }
}

fn build_node(text: String) -> Node {
    Node {
        element: Element {
            body: ElementBody::Text(crate::render::Text { text, size: 24.0 }),
            style: Style::default(),
        },
        events: ElementEventHandlers::default(),
    }
}
