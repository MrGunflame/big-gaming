mod button;
mod text;

pub use button::Button;

use crate::events::EventHandlers;
use crate::render::Element;

pub trait BuildWidget {
    fn build(self) -> Widget;
}

pub struct Widget {
    pub element: Element,
    pub events: EventHandlers,
}
