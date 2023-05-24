use game_input::mouse::MouseButtonInput;

use crate::events::EventHandlers;
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
            events: EventHandlers {
                mouse_button_input: Some(input_handler(props.on_click.0)),
                ..Default::default()
            },
        })
    }
}

fn input_handler(
    f: Box<dyn Fn() + Send + Sync + 'static>,
) -> Box<dyn Fn(MouseButtonInput) + Send + Sync + 'static> {
    Box::new(move |event| {
        if event.button.is_left() && event.state.is_pressed() {
            f();
        }
    })
}

pub struct ButtonHandler(Box<dyn Fn() + Send + Sync + 'static>);

impl Default for ButtonHandler {
    fn default() -> Self {
        Self(Box::new(|| {}))
    }
}

impl<F> From<F> for ButtonHandler
where
    F: Fn() + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}
