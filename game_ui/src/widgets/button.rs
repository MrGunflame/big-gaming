use game_input::mouse::MouseButtonInput;

use crate::events::EventHandlers;
use crate::render::layout::Key;
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::text::Text;
use super::{Context, Widget};

#[derive(Default)]
pub struct Button {
    pub onclick: Option<Box<dyn Fn() + Send + Sync + 'static>>,
    pub style: Style,
}

impl Widget for Button {
    fn create(self, ctx: &mut Context<'_>) -> Key {
        let elem = Element {
            body: ElementBody::Container(),
            style: self.style,
        };

        let key = ctx.tree.push(ctx.parent, elem);
        ctx.events.insert(
            key,
            EventHandlers {
                mouse_button_input: self.onclick.map(|f| click_handler(f)),
                ..Default::default()
            },
        );

        key
    }
}

fn click_handler(
    f: Box<dyn Fn() + Send + Sync + 'static>,
) -> Box<dyn Fn(MouseButtonInput) + Send + Sync + 'static> {
    Box::new(move |input| {
        if input.state.is_pressed() && input.button.is_left() {
            f();
        }
    })
}

#[derive(Default)]
pub struct LabeledButton {
    pub text: String,
    pub size: f32,
    pub onclick: Option<Box<dyn Fn() + Send + Sync + 'static>>,
}

impl Widget for LabeledButton {
    fn create(self, ctx: &mut Context<'_>) -> Key {
        let key = Button {
            onclick: self.onclick,
            style: Style::default(),
        }
        .create(ctx);

        let mut ctx = Context {
            parent: Some(key),
            tree: ctx.tree,
            events: ctx.events,
        };

        Text {
            text: self.text,
            size: self.size,
        }
        .create(&mut ctx)
    }
}
