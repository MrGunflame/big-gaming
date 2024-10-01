use std::convert::Infallible;

use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;
use game_window::events::CursorMoved;

use crate::reactive::Context;
use crate::runtime_v2::View;
use crate::style::{Background, Style};

use super::container::Container2;
use super::{Callback, Container, Widget};

const DEFAULT_ON_HOVER_COLOR: Background = Background::TEAL;

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
        let _span = trace_span!("Button::mount").entered();

        let wrapper = Container::new().style(self.style).mount(parent);

        parent.document().register_with_parent(
            wrapper.node.unwrap(),
            move |ctx: Context<MouseButtonInput>| {
                if !ctx.event.button.is_left() || !ctx.event.state.is_pressed() {
                    return;
                }

                if let Some(layout) = ctx.layout(wrapper.node.unwrap()) {
                    if layout.contains(ctx.cursor().as_uvec2()) {
                        self.on_click.call(());
                    }
                }
            },
        );

        parent.document().register_with_parent(
            wrapper.node.unwrap(),
            move |ctx: Context<CursorMoved>| {
                if let Some(layout) = ctx.layout(wrapper.node.unwrap()) {
                    if layout.contains(ctx.cursor().as_uvec2()) {}
                }
            },
        );

        wrapper
    }
}

impl crate::runtime_v2::Widget for Button {
    type Message = Infallible;

    fn render(&self, ctx: &crate::runtime_v2::Context<Self>) -> crate::runtime_v2::View {
        Container2::new().into()
    }
}
