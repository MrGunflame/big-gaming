use std::fmt::Display;
use std::str::FromStr;

use glam::UVec2;

use crate::events::{ElementEventHandlers, EventHandlers};
use crate::layout::{Element, ElementBody};
use crate::reactive::{Node, Scope};
use crate::style::Style;

use super::{Callback, Text, Widget};

pub trait SlidingValue: FromStr + Display + Clone {
    fn step_forward(&mut self);
    fn step_backward(&mut self);
}

impl SlidingValue for f32 {
    fn step_forward(&mut self) {
        *self += 1.0;
    }

    fn step_backward(&mut self) {
        *self += 1.0;
    }
}

#[derive(Clone, Debug, Default)]
pub struct ValueSlider<T>
where
    T: SlidingValue,
{
    value: T,
    style: Style,
    on_change: Option<Callback<T>>,
}

impl<T> ValueSlider<T>
where
    T: SlidingValue,
{
    pub fn new(value: T) -> Self {
        Self {
            value,
            style: Style::default(),
            on_change: None,
        }
    }

    pub fn value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn on_change<F>(mut self, on_change: F) -> Self
    where
        F: Into<Callback<T>>,
    {
        self.on_change = Some(on_change.into());
        self
    }
}

impl<T> Widget for ValueSlider<T>
where
    T: SlidingValue + Send + Sync + 'static,
{
    fn build(self, cx: &Scope) -> Scope {
        let (value, set_value) = cx.create_signal(self.value);
        let (state, set_state) = cx.create_signal(State::default());

        let root = cx.push(Node {
            element: Element {
                body: ElementBody::Container,
                style: self.style,
            },
            events: ElementEventHandlers {
                local: EventHandlers {
                    cursor_moved: Some(Box::new(move |ctx| {
                        let state = state.get();

                        if !state.enabled {
                            return;
                        }

                        if ctx.event.position.x as u32 > state.position.x {
                            set_value.update(|value| value.step_forward());
                        } else if (ctx.event.position.x as u32) < state.position.x {
                            set_value.update(|value| value.step_backward());
                        }
                    })),
                    mouse_button_input: Some(Box::new(move |ctx| {
                        if !ctx.event.button.is_left() {
                            return;
                        }

                        set_state.update(|state| state.enabled = ctx.event.state.is_pressed());
                    })),
                    ..Default::default()
                },
                ..Default::default()
            },
        });

        let text = root.append(Text::new());
        let mut id = text.id().unwrap();
        let root2 = root.clone();
        cx.create_effect(move || {
            let value = value.get();

            text.remove(id);
            let cx = root2.append(Text::new().text(value.to_string()));
            id = cx.id().unwrap();
        });

        root
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct State {
    enabled: bool,
    position: UVec2,
}
