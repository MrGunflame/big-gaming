use std::marker::PhantomData;

use game_input::mouse::MouseButtonInput;

use crate::events::EventHandlers;
use crate::reactive::{Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

#[derive(Default)]
pub struct Props<F> {
    pub onclick: F,
}

pub struct Button<F> {
    _marker: PhantomData<F>,
}

use super::Widget;

impl<F> Widget for Button<F>
where
    F: Fn() + Send + Sync + 'static,
{
    type Properties = Props<F>;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        cx.push(Node {
            element: Element {
                body: ElementBody::Container(),
                style: Style::default(),
            },
            events: EventHandlers {
                mouse_button_input: Some(input_handler(props.onclick)),
                ..Default::default()
            },
        })
    }
}

pub fn Button<F>(cx: &Scope, on_click: F) -> Scope
where
    F: Fn() + Send + Sync + 'static,
{
    cx.push(Node {
        element: Element {
            body: ElementBody::Container(),
            style: Style::default(),
        },
        events: EventHandlers {
            mouse_button_input: Some(input_handler(on_click)),
            ..Default::default()
        },
    })
}

fn input_handler(
    f: impl Fn() + Send + Sync + 'static,
) -> Box<dyn Fn(MouseButtonInput) + Send + Sync + 'static> {
    Box::new(move |event| {
        if event.button.is_left() && event.state.is_pressed() {
            f();
        }
    })
}
