use crate::events::EventHandlers;
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::{BuildWidget, Widget};

#[derive(Default)]
pub struct Button {
    pub onclick: Option<Box<dyn Fn()>>,
}

impl BuildWidget for Button {
    fn build(self) -> Widget {
        Widget {
            element: Element {
                body: ElementBody::Container(),
                style: Style::default(),
            },
            events: EventHandlers::default(),
        }
    }
}
