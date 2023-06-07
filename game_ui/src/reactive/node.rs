use crate::events::ElementEventHandlers;
use crate::render::Element;

#[derive(Debug)]
pub struct Node {
    pub element: Element,
    pub events: ElementEventHandlers,
}
