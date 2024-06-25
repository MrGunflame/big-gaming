use image::{ImageBuffer, Rgba};

use crate::layout::{Element, ElementBody};
use crate::render::Text;
use crate::style::Style;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Clone, Debug)]
pub struct Primitive {
    pub style: Style,
    pub image: Option<Image>,
    pub text: Option<Text>,
}

// tmp
impl From<Primitive> for Element {
    fn from(value: Primitive) -> Self {
        match (value.image, value.text) {
            (Some(image), _) => Self {
                style: value.style,
                body: ElementBody::Image(crate::render::Image { image }),
            },
            (_, Some(text)) => Self {
                style: value.style,
                body: ElementBody::Text(text),
            },
            _ => Self {
                style: value.style,
                body: ElementBody::Container,
            },
        }
    }
}
