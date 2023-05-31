use image::{ImageBuffer, Rgba};

use crate::events::EventHandlers;
use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{self, Element, ElementBody};

use super::Component;

pub struct Image;

pub struct ImageProps {
    pub image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub style: Style,
}

impl Component for Image {
    type Properties = ImageProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        let cx = cx.push(Node {
            element: Element {
                body: ElementBody::Image(render::Image { image: props.image }),
                style: props.style,
            },
            events: EventHandlers::default(),
        });

        cx
    }
}
