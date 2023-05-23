use crate::events::EventHandlers;
use crate::render::Element;

pub struct Node {
    pub element: Element,
    pub events: EventHandlers,
}
