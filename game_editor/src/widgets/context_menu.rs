//! Context-menu (right-click)

use game_input::mouse::MouseButtonInput;
use game_ui::reactive::{Context, NodeId};

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

        if let Some(state) = parent.document().get::<InternalContextMenuState>() {
            state
                .parents
                .lock()
                .push((wrapper.clone(), self.spawn_menu));
        } else {
            let state = InternalContextMenuState::default();
            state
                .parents
                .lock()
                .push((wrapper.clone(), self.spawn_menu));
            parent.document().insert(state);

            parent
                .document()
                .register(move |ctx: Context<MouseButtonInput>| {
                    if !ctx.event.state.is_pressed() {
                        return;
                    }

                    let cursor = ctx.cursor().as_uvec2();
                    let Some(state) = ctx.document().get::<InternalContextMenuState>() else {
                        return;
                    };

                    // - If the context menu is open and a click originated inside the
                    // context menu do nothing.
                    // - If the context menu is open and a click originated outside the
                    // context menu close the context menu.
                    // - If no context menu is open do nothing.
                    let mut active = state.active.lock();
                    if let Some(node) = *active {
                        if ctx.layout(node).unwrap().contains(cursor) {
                            return;
                        }

                        ctx.runtime().remove(node);
                        *active = None;
                    }

                    if !ctx.event.button.is_right() {
                        return;
                    }

                    let parents = state.parents.lock();
                    for (node, cb) in parents.iter() {
                        let Some(layout) = ctx.layout(node.node().unwrap()) else {
                            continue;
                        };

                        if layout.contains(cursor) {
                            *active = Some(node.node().unwrap());

                            let ctx2 = ctx.clone();
                            let closer = Callback::from(move |()| {
                                let Some(state) = ctx2.document().get::<InternalContextMenuState>()
                                else {
                                    return;
                                };

                                if let Some(node) = state.active.lock().take() {
                                    ctx2.runtime().remove(node);
                                };
                            });

                            let menu_ctx = Container::new()
                                .style(Style {
                                    position: Position::Absolute(cursor),
                                    ..Default::default()
                                })
                                .mount(node);

                            *active = Some(menu_ctx.node().unwrap());
                            let cb = cb.clone();
                            drop(parents);
                            drop(active);

                            cb.call(ContextMenuState {
                                ctx: menu_ctx,
                                closer: ContextMenuCloser { closer },
                            });

                            break;
                        }
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

#[derive(Debug, Default)]
pub struct InternalContextMenuState {
    parents: Mutex<Vec<(Context<()>, Callback<ContextMenuState>)>>,
    active: Mutex<Option<NodeId>>,
}
