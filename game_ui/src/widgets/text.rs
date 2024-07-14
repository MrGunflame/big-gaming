use game_tracing::trace_span;

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::Style;

use super::Widget;

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub size: f32,
    caret: Option<u32>,
}

impl Text {
    pub fn new<T>(text: T) -> Self
    where
        T: ToString,
    {
        Self {
            text: text.to_string(),
            size: 24.0,
            caret: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub(crate) fn caret(mut self, caret: Option<u32>) -> Self {
        self.caret = caret;
        self
    }
}

impl Widget for Text {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let _span = trace_span!("Text::mount").entered();

        parent.append(Node::new(Primitive {
            style: Style::default(),
            image: None,
            text: Some(crate::render::Text {
                text: self.text,
                size: self.size,
                caret: self.caret,
            }),
        }))
    }
}
