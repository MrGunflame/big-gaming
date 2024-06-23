//! Context-menu (right-click)

use std::sync::Arc;

use game_input::mouse::MouseButtonInput;
use game_ui::reactive::Context;

use game_ui::style::{Position, Style};
use game_ui::widgets::{Callback, Container, Widget};
use parking_lot::Mutex;

/// A transparent wrapper around a container that passes requests for context menus along.
///
/// `spawn_menu` is called when a right-click is issued (when the context menu is requested).
#[derive(Debug)]
pub struct ContextPanel {
    spawn_menu: Callback<ContextMenuState>,
    style: Style,
}

impl ContextPanel {
    pub fn new() -> Self {
        Self {
            spawn_menu: Callback::from(|_| {}),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn spawn_menu<F>(mut self, f: F) -> Self
    where
        F: Into<Callback<ContextMenuState>>,
    {
        self.spawn_menu = f.into();
        self
    }
}

impl Widget for ContextPanel {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let wrapper = Container::new().mount(parent);

        let context_menu = Arc::new(Mutex::new(None));

        {
            let wrapper = wrapper.clone();
            parent
                .document()
                .register(move |ctx: Context<MouseButtonInput>| {
                    let cursor = ctx.cursor().as_uvec2();

                    if let Some(node) = *context_menu.lock() {
                        if ctx.layout(node).unwrap().contains(cursor) {
                            return;
                        }

                        ctx.runtime().remove(node);
                        *context_menu.lock() = None;
                    }

                    // FIXME: Might race?
                    debug_assert!(context_menu.lock().is_none());

                    if ctx
                        .layout(wrapper.node().unwrap())
                        .unwrap()
                        .contains(cursor)
                    {
                        let context_menu = context_menu.clone();
                        let closer = Callback::from(move |()| {
                            if let Some(node) = context_menu.lock().take() {
                                ctx.runtime().remove(node);
                            }
                        });

                        let context_menu = Container::new()
                            .style(Style {
                                position: Position::Absolute(cursor),
                                ..Default::default()
                            })
                            .mount(&wrapper);
                        self.spawn_menu.call(ContextMenuState {
                            ctx: context_menu,
                            closer: ContextMenuCloser { closer },
                        });
                    }
                });
        }

        wrapper
    }
}

#[derive(Clone, Debug)]
pub struct ContextMenuState {
    pub ctx: Context<()>,
    pub closer: ContextMenuCloser,
}

#[derive(Clone, Debug)]
pub struct ContextMenuCloser {
    closer: Callback<()>,
}

impl ContextMenuCloser {
    pub fn close(&self) {
        self.closer.call(());
    }
}
