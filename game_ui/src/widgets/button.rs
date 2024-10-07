use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;
use wgpu::hal::auxil::db;

use crate::runtime::Context;
use crate::style::Style;

use super::{Callback, Container, Widget};

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
    fn mount(self, parent: &Context) -> Context {
        let _span = trace_span!("Button::mount").entered();

        let wrapper = Container::new().style(self.style).mount(parent);

        dbg!(&wrapper.node());
        let ctx = wrapper.clone();
        parent.document().register_with_parent(
            wrapper.node().unwrap(),
            move |event: MouseButtonInput| {
                dbg!(&event);
                if !event.button.is_left() || !event.state.is_pressed() {
                    return;
                }

                if let Some(layout) = ctx.layout(ctx.node().unwrap()) {
                    if layout.contains(ctx.cursor().as_uvec2()) {
                        self.on_click.call(());
                    }
                } else {
                    dbg!("no layout");
                }
            },
        );

        wrapper
    }
}
