use std::convert::Infallible;

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::runtime_v2::View;
use crate::style::Style;

use super::Widget;

#[derive(Clone, Debug, Default)]
pub struct Container {
    style: Style,
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Container {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        parent.append(Node::new(Primitive {
            style: self.style,
            image: None,
            text: None,
        }))
    }
}

pub struct Container2 {
    style: Style,
    content: View,
}

impl Container2 {
    pub fn new(content: View) -> Self {
        Self {
            content,
            style: Style::default(),
        }
    }
}

impl crate::runtime_v2::Widget for Container2 {
    type Message = Infallible;

    fn render(&self, _ctx: &crate::runtime_v2::Context<Self>) -> View {
        View::Container(
            Primitive {
                style: self.style.clone(),
                image: None,
                text: None,
            },
            Box::new(self.content.clone()),
        )
    }
}
