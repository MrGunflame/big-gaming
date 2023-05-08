use crate::events::EventHandlers;
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::{BuildWidget, Widget};

pub struct Text {
    pub text: String,
    pub size: f32,
}

impl BuildWidget for Text {
    fn build(self) -> super::Widget {
        Widget {
            element: Element {
                body: ElementBody::Text(crate::render::Text {
                    text: self.text,
                    size: self.size,
                }),
                style: Style::default(),
            },
            events: EventHandlers::default(),
        }
    }
}
