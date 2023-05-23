use std::sync::Arc;

use crate::events::EventHandlers;
use crate::reactive::{create_effect, Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody, Text};

pub fn Text<T>(cx: &Scope, text: T) -> Scope
where
    T: Into<TextProp>,
{
    let cx = cx.push(Node {
        element: Element {
            body: ElementBody::Text(Text {
                text: "text".to_owned(),
                size: 24.0,
            }),
            style: Style::default(),
        },
        events: EventHandlers::default(),
    });

    let text = text.into();

    let cx2 = cx.clone();
    let id = cx.id().unwrap();
    create_effect(&cx, move |_| {
        let string = (text.0)();

        cx2.update(
            id,
            Node {
                element: Element {
                    body: ElementBody::Text(Text {
                        text: string,
                        size: 24.0,
                    }),
                    style: Style::default(),
                },
                events: EventHandlers::default(),
            },
        );
    });

    cx
}

#[derive(Clone)]
pub struct TextProp(Arc<dyn Fn() -> String + Send + Sync + 'static>);

impl From<String> for TextProp {
    fn from(value: String) -> Self {
        TextProp(Arc::new(move || value.clone()))
    }
}

impl<F> From<F> for TextProp
where
    F: Fn() -> String + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        TextProp(Arc::new(value))
    }
}

impl From<&str> for TextProp {
    fn from(value: &str) -> Self {
        let s = value.to_owned();
        s.into()
    }
}
