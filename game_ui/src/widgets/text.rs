use std::sync::Arc;

use crate::events::EventHandlers;
use crate::reactive::{create_effect, Node, Scope};
use crate::render::style::Style;
use crate::render::{Element, ElementBody};

use super::Component;

#[derive(Default)]
pub struct TextProps {
    pub text: TextProp,
}

pub struct Text;

impl Component for Text {
    type Properties = TextProps;

    fn render(cx: &Scope, props: Self::Properties) -> Scope {
        let text = props.text.clone();

        let cx = cx.push(Node {
            element: Element {
                body: ElementBody::Text(crate::render::Text {
                    text: (text.0)(),
                    size: 24.0,
                }),
                style: Style::default(),
            },
            events: EventHandlers::default(),
        });

        let cx2 = cx.clone();
        let id = cx.id().unwrap();
        create_effect(&cx, move |_| {
            let string = (text.0)();

            cx2.update(
                id,
                Node {
                    element: Element {
                        body: ElementBody::Text(crate::render::Text {
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

impl Default for TextProp {
    fn default() -> Self {
        Self::from("")
    }
}
