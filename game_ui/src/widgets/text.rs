use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::Style;

use super::Widget;

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub size: f32,
}

impl Text {
    pub fn new<T>(text: T) -> Self
    where
        T: ToString,
    {
        Self {
            text: text.to_string(),
            size: 16.0,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl Widget for Text {
    fn mount<T>(self, parent: &Context<T>) {
        parent.append(Node::new(Primitive {
            style: Style::default(),
            image: None,
            text: Some(crate::render::Text {
                text: self.text,
                size: self.size,
            }),
        }));
    }
}
