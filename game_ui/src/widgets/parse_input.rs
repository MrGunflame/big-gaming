use std::fmt::Display;
use std::str::FromStr;

use crate::reactive::Scope;
use crate::style::Style;

use super::{Callback, Input, ValueProvider, Widget};

#[derive(Clone, Debug, Default)]
pub struct ParseInput<T>
where
    T: Send + Sync + 'static,
{
    value: ValueProvider<T>,
    style: Style,
    on_change: Option<Callback<T>>,
}

impl<T> ParseInput<T>
where
    T: Send + Sync + 'static,
{
    pub fn new<P>(value: P) -> Self
    where
        P: Into<ValueProvider<T>>,
    {
        Self {
            value: value.into(),
            style: Style::default(),
            on_change: None,
        }
    }

    pub fn value<P>(mut self, value: P) -> Self
    where
        P: Into<ValueProvider<T>>,
    {
        self.value = value.into();
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

impl<T> Widget for ParseInput<T>
where
    T: Send + Sync + Display + FromStr + 'static,
{
    fn build(self, cx: &Scope) -> Scope {
        let (value, set_value) = cx.create_signal(self.value.with(T::to_string));

        if let ValueProvider::Reader(reader) = self.value {
            cx.create_effect(move || {
                let string = reader.with(T::to_string);
                set_value.set(string);
            });
        }

        let mut input = Input::new()
            .value(ValueProvider::Reader(value))
            .style(self.style);

        if let Some(cb) = self.on_change {
            let on_change = move |value: String| {
                if let Ok(value) = value.parse::<T>() {
                    (cb)(value);
                }
            };

            input = input.on_change(on_change);
        }

        input.build(cx)
    }
}
