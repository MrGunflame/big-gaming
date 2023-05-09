mod button;
mod text;

pub use button::{Button, LabeledButton};
pub use text::Text;

use crate::events::Events;
use crate::render::layout::{Key, LayoutTree};

pub struct Context<'a> {
    pub parent: Option<Key>,
    pub tree: &'a mut LayoutTree,
    pub events: &'a mut Events,
}

impl<'a> Context<'a> {
    pub fn child<'b, 'c>(&'b mut self, parent: Key) -> Context<'c>
    where
        'a: 'c,
        'b: 'c,
    {
        Context {
            parent: Some(parent),
            tree: self.tree,
            events: self.events,
        }
    }
}

pub trait Widget {
    fn create(self, ctx: &mut Context<'_>) -> Key;
}
