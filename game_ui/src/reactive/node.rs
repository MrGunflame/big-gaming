use crate::events::EventHandlers;
use crate::render::Element;

#[derive(Debug)]
pub struct Node {
    pub element: Element,
    pub events: EventHandlers,
}
