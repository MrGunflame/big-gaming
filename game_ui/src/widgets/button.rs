use game_input::mouse::MouseButtonInput;

use crate::events::{Context, ElementEventHandlers, EventHandlers};
use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::Component;

#[derive(Default)]
pub struct ButtonProps {
    pub on_click: ButtonHandler,
    pub style: Style,
}

pub struct Button;

impl Component for Button {
    type Properties = ButtonProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: props.style,
            },
            events: ElementEventHandlers {
                local: EventHandlers {
                    mouse_button_input: Some(input_handler(props.on_click.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
        })
    }
}

fn input_handler(
    f: Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |ctx| {
        if ctx.event.button.is_left() && ctx.event.state.is_pressed() {
            f(ctx);
        }
    })
}

pub struct ButtonHandler(Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>);

impl Default for ButtonHandler {
    fn default() -> Self {
        Self(Box::new(|_| {}))
    }
}

impl<F> From<F> for ButtonHandler
where
    F: Fn(Context<MouseButtonInput>) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}
