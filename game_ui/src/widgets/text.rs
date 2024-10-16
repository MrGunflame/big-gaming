use std::borrow::Cow;
use std::ops::Range;

use game_tracing::trace_span;

use crate::primitive::Primitive;
use crate::runtime::Context;
use crate::style::{Color, Style};

use super::Widget;

#[derive(Clone, Debug)]
pub struct Text {
    text: String,
    size: f32,
    caret: Option<u32>,
    selection_range: Option<Range<usize>>,
    selection_color: Color,
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
            selection_range: None,
            selection_color: Color::BLACK,
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

    pub(crate) fn selection_range(mut self, selection_range: Option<Range<usize>>) -> Self {
        self.selection_range = selection_range;
        self
    }

    pub(crate) fn selection_color(mut self, selection_color: Color) -> Self {
        self.selection_color = selection_color;
        self
    }
}

impl Widget for Text {
    fn mount(self, parent: &Context) -> Context {
        let _span = trace_span!("Text::mount").entered();

        parent.append(Primitive {
            style: Style::default(),
            image: None,
            text: Some(crate::render::Text {
                text: Cow::Owned(self.text),
                size: self.size,
                caret: self.caret,
                selection_range: self.selection_range,
                selection_color: self.selection_color,
            }),
        })
    }
}
