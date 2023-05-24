use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::Component;

#[derive(Default)]
pub struct ContainerProps {
    pub style: Style,
}

pub struct Container;

impl Component for Container {
    type Properties = ContainerProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: props.style,
            },
            events: Default::default(),
        })
    }
}
