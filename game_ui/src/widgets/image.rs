use std::convert::Infallible;

use game_tracing::trace_span;
use image::{ImageBuffer, Rgba};

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::Style;

use super::Widget;

pub struct Image {
    image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    style: Style,
}

impl Image {
    pub fn new() -> Self {
        Self {
            image: ImageBuffer::new(0, 0),
            style: Style::default(),
        }
    }

    pub fn image(mut self, image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        self.image = image;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Default for Image {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Image {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let _span = trace_span!("Image::mount").entered();

        parent.append(Node::new(Primitive {
            style: self.style,
            image: Some(self.image),
            text: None,
        }))
    }
}

impl crate::runtime_v2::Widget for Image {
    type Message = Infallible;

    fn render(&self, _ctx: &crate::runtime_v2::Context<Self>) -> crate::runtime_v2::View {
        crate::runtime_v2::View::Primitive(Primitive {
            style: self.style.clone(),
            image: Some(self.image.clone()),
            text: None,
        })
    }
}
