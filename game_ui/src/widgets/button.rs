use game_input::mouse::MouseButtonInput;

use crate::events::{Context, ElementEventHandlers, EventHandlers};
use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::{Callback, Widget};

#[derive(Debug, Default)]
pub struct Button {
    style: Style,
    on_click: Option<Callback<Context<MouseButtonInput>>>,
}

impl Button {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn on_click<T>(mut self, on_click: T) -> Self
    where
        T: Into<Callback<Context<MouseButtonInput>>>,
    {
        self.on_click = Some(on_click.into());
        self
    }
}

impl Widget for Button {
    fn build(self, cx: &Scope) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: self.style,
            },
            events: ElementEventHandlers {
                local: EventHandlers {
                    mouse_button_input: self.on_click.map(|f| input_handler(f)),
                    ..Default::default()
                },
                ..Default::default()
            },
        })
    }
}

fn input_handler(
    on_click: Callback<Context<MouseButtonInput>>,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |ctx| {
        if ctx.event.button.is_left() && ctx.event.state.is_pressed() {
            on_click(ctx);
        }
    })
}
