use crate::primitive::Primitive;
use crate::runtime::Context;
use crate::style::Style;

use super::Widget;

#[derive(Clone, Debug, Default)]
pub struct Container {
    style: Style,
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Container {
    fn mount(self, parent: &Context) -> Context {
        parent.append(Primitive {
            style: self.style,
            image: None,
            text: None,
        })
    }
}
