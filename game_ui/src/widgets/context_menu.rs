//! Context-menu (right-click)

use game_input::mouse::MouseButtonInput;
use parking_lot::Mutex;

use crate::reactive::{Context, NodeDestroyed, NodeId};
use crate::style::{Position, Style};
use crate::widgets::{Callback, Container, Widget};

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

        // Cleanup when the parent is removed.
        parent.document().register_with_parent(
            wrapper.node().unwrap(),
            move |ctx: Context<NodeDestroyed>| {
                let node = ctx.node().unwrap();
                let state = ctx.document().get::<InternalContextMenuState>().unwrap();

                // If the context menu is still open we should
                // close it before we remove it.
                let mut active = state.active.try_lock().unwrap();
                if *active == Some(node) {
                    ctx.runtime().remove(node);
                    *active = None;
                }

                let mut parents = state.parents.try_lock().unwrap();
                parents.retain(|(p_ctx, _)| p_ctx.node().unwrap() != node);

                // We just destroyed the last context menu parent element.
                // Remove the remaining state from the document.
                if parents.is_empty() {
                    ctx.document().remove::<InternalContextMenuState>();
                }
            },
        );

        if let Some(state) = parent.document().get::<InternalContextMenuState>() {
            state
                .parents
                .try_lock()
                .unwrap()
                .push((wrapper.clone(), self.spawn_menu));
        } else {
            let state = InternalContextMenuState::default();
            state
                .parents
                .try_lock()
                .unwrap()
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
                    let mut active = state.active.try_lock().unwrap();
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

                    let parents = state.parents.try_lock().unwrap();
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

                                if let Some(node) = state.active.try_lock().unwrap().take() {
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
struct InternalContextMenuState {
    parents: Mutex<Vec<(Context<()>, Callback<ContextMenuState>)>>,
    active: Mutex<Option<NodeId>>,
}
