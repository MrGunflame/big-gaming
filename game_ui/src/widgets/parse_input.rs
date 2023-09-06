use std::fmt::Display;
use std::str::FromStr;

use crate::reactive::Scope;
use crate::style::Style;

use super::{Callback, Input, Widget};

#[derive(Clone, Debug, Default)]
pub struct ParseInput<T> {
    value: T,
    style: Style,
    on_change: Option<Callback<T>>,
}

impl<T> ParseInput<T> {
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

impl<T> Widget for ParseInput<T>
where
    T: Display + FromStr + 'static,
{
    fn build(self, cx: &Scope) -> Scope {
        let value = self.value.to_string();
        let style = self.style;

        let mut input = Input::new().value(value).style(style);

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
