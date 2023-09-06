use image::{ImageBuffer, Rgba};

use crate::events::ElementEventHandlers;
use crate::reactive::{Node, Scope};
use crate::render::{self, Element, ElementBody};
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

impl Widget for Image {
    fn build(self, cx: &Scope) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Image(render::Image { image: self.image }),
                style: self.style,
            },
            events: ElementEventHandlers::default(),
        })
    }
}
