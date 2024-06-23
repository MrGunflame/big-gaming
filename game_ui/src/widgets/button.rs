use game_input::mouse::MouseButtonInput;

use crate::primitive::Primitive;
use crate::reactive::{Context, Node};
use crate::style::Style;

use super::{Callback, Widget};

pub struct Button {
    pub style: Style,
    pub on_click: Callback<()>,
}

impl Button {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            on_click: Callback::default(),
        }
    }

    pub fn on_click<T>(mut self, on_click: T) -> Self
    where
        T: Into<Callback<()>>,
    {
        self.on_click = on_click.into();
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Button {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let mut node = Node::new(Primitive {
            style: self.style,
            image: None,
            text: None,
        });
        node.register(move |ctx: Context<MouseButtonInput>| {
            if ctx.event.button.is_left() && ctx.event.state.is_pressed() {
                self.on_click.call(());
            }
        });

        parent.append(node)
    }
}
