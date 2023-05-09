use crate::events::EventHandlers;
use crate::render::layout::Key;
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::{Context, Widget};

pub struct Text {
    pub text: String,
    pub size: f32,
}

impl Widget for Text {
    fn create(self, ctx: &mut Context<'_>) -> Key {
        let elem = Element {
            body: ElementBody::Text(crate::render::Text::new(self.text, self.size)),
            style: Style::default(),
        };

        let key = ctx.tree.push(ctx.parent, elem);
        key
    }
}
