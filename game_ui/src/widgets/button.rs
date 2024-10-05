use std::cell::OnceCell;

use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;
use game_window::events::CursorMoved;

use crate::reactive::Context;
use crate::runtime_v2::{Children, EventHandlerHandle, NodeRef};
use crate::style::{Background, Style};

use super::container::Container2;
use super::{Callback, Container, Widget};

const DEFAULT_ON_HOVER_COLOR: Background = Background::TEAL;

pub struct Button {
    style: Style,
    on_click: Callback<()>,
    state: OnceCell<ButtonState>,
    content: Children,
}

impl Button {
    pub fn new<T>(content: T) -> Self
    where
        T: Into<Children>,
    {
        Self {
            style: Style::default(),
            on_click: Callback::default(),
            state: OnceCell::new(),
            content: content.into(),
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
    type Message = Message;

    fn update(&mut self, ctx: &crate::runtime_v2::Context<Self>, msg: Self::Message) -> bool {
        let Some(state) = self.state.get() else {
            return false;
        };

        match msg {
            Message::MouseButtonInput(event) => {
                if !event.button.is_left() || !event.state.is_pressed() {
                    return false;
                }

                let Some(layout) = ctx.layout(&state.node_ref) else {
                    return false;
                };

                let Some(cursor) = ctx.cursor().position() else {
                    return false;
                };

                if layout.contains(cursor) {
                    self.on_click.call(());
                }
            }
        }

        false
    }

    fn view(&self, ctx: &crate::runtime_v2::Context<Self>) -> crate::runtime_v2::View {
        let state = self.state.get_or_init(|| {
            let node_ref = ctx.create_node_ref();

            let mouse_button_input = ctx.on_event(Message::MouseButtonInput);

            ButtonState {
                node_ref,
                _mouse_button_input: mouse_button_input,
            }
        });

        Container2::new(self.content.clone())
            .node_ref(state.node_ref.clone())
            .into()
    }
}

pub enum Message {
    MouseButtonInput(MouseButtonInput),
}

#[derive(Debug)]
struct ButtonState {
    node_ref: NodeRef,
    _mouse_button_input: EventHandlerHandle,
}
