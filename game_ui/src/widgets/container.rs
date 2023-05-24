use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

pub fn Container(cx: &Scope, style: Style) -> Scope {
    cx.push(Node {
        element: Element {
            body: ElementBody::Container(),
            style,
        },
        events: Default::default(),
    })
}
