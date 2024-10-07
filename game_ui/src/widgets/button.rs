use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;

use crate::reactive::Context;
use crate::style::Style;

use super::{Callback, Container, MountEffect, Widget, Widget2};

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

        wrapper
    }
}

impl Widget2 for Button {
    fn mount(self, ctx: &crate::runtime::Context) -> crate::runtime::Context {
        let (state, set_state) = ctx.reactive().create_signal(false);

        let root = Container::new().mount(ctx);

        ctx.reactive().register_effect(&[state.id()], {
            let root = root.clone();
            move || {
                root.clear_children();
            }
        });

        root
    }
}
