use crate::events::EventHandlers;
use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody, Text};

pub fn Text(cx: &Scope, text: &str) -> Scope {
    cx.push(Node {
        element: Element {
            body: ElementBody::Text(Text {
                text: text.to_owned(),
                size: 24.0,
            }),
            style: Style::default(),
        },
        events: EventHandlers::default(),
    })
}
