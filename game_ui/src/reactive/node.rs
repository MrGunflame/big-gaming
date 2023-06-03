use crate::events::{ElementEventHandlers, EventHandlers};
use crate::render::Element;

#[derive(Debug)]
pub struct Node {
    pub element: Element,
    pub events: ElementEventHandlers,
}
