use std::convert::Infallible;

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::runtime_v2::{Children, NodeRef, View};
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
    content: Children,
    node_ref: Option<NodeRef>,
}

impl Container2 {
    pub fn new<T>(content: T) -> Self
    where
        T: Into<Children>,
    {
        Self {
            content: content.into(),
            style: Style::default(),
            node_ref: None,
        }
    }

    pub fn node_ref(mut self, node_ref: NodeRef) -> Self {
        self.node_ref = Some(node_ref);
        self
    }
}

impl crate::runtime_v2::Widget for Container2 {
    type Message = Infallible;

    fn view(&self, _ctx: &crate::runtime_v2::Context<Self>) -> View {
        View {
            primitive: Some(Primitive {
                style: self.style.clone(),
                image: None,
                text: None,
            }),
            children: self.content.clone(),
            node_ref: self.node_ref.clone(),
        }
    }
}
