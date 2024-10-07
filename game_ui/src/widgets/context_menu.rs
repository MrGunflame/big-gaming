//! Context-menu (right-click)

use game_input::mouse::MouseButtonInput;
use parking_lot::Mutex;

use crate::runtime::events::NodeDestroyed;
use crate::runtime::{Context, NodeId};
use crate::style::{Position, Style};
use crate::widgets::{Callback, Container, Widget};

/// A transparent wrapper around a container that passes requests for context menus along.
///
/// `spawn_menu` is called when a right-click is issued (when the context menu is requested).
#[derive(Debug)]
pub struct ContextPanel {
    spawn_menu: Callback<ContextMenuState>,
    style: Style,
    priority: u32,
}

impl ContextPanel {
    pub fn new() -> Self {
        Self {
            spawn_menu: Callback::from(|_| {}),
            style: Style::default(),
            priority: 0,
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

    /// Sets the priority of this `ContextPanel`.
    ///
    /// If multiple `ContextPanel`s are clicked at the same time the panel with the highest
    /// priority is chosen. If the priority is equal for all panels any panel may be chosen.
    ///
    /// Defaults to `0`.
    #[inline]
    #[must_use]
    pub fn priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

impl Widget for ContextPanel {
    fn mount(self, parent: &Context) -> Context {
        let wrapper = Container::new().style(self.style).mount(parent);

        // Cleanup when the parent is removed.
        parent
            .document()
            .register_with_parent(wrapper.node().unwrap(), {
                let ctx = wrapper.clone();
                move |event: NodeDestroyed| {
                    let node = ctx.node().unwrap();
                    let state = ctx.document().get::<InternalContextMenuState>().unwrap();

                    // If the context menu is still open we should
                    // close it before we remove it.
                    let mut active = state.active.try_lock().unwrap();
                    if *active == Some(node) {
                        ctx.remove(node);
                        *active = None;
                    }

                    let mut parents = state.parents.try_lock().unwrap();
                    parents.retain(|item| item.node.node().unwrap() != node);
                    parents.sort_by(|a, b| a.priority.cmp(&b.priority).reverse());

                    // We just destroyed the last context menu parent element.
                    // Remove the remaining state from the document.
                    if parents.is_empty() {
                        ctx.document().remove::<InternalContextMenuState>();
                    }
                }
            });

        if let Some(state) = parent.document().get::<InternalContextMenuState>() {
            let mut parents = state.parents.try_lock().unwrap();
            parents.push(ContextMenuItem {
                node: wrapper.clone(),
                callback: self.spawn_menu,
                priority: self.priority,
            });
            parents.sort_by(|a, b| a.priority.cmp(&b.priority).reverse());
        } else {
            let state = InternalContextMenuState::default();
            let mut parents = state.parents.try_lock().unwrap();
            parents.push(ContextMenuItem {
                node: wrapper.clone(),
                callback: self.spawn_menu,
                priority: self.priority,
            });
            parents.sort_by(|a, b| a.priority.cmp(&b.priority).reverse());
            drop(parents);

            parent.document().insert(state);

            let ctx = wrapper.clone();
            parent.document().register(move |event: MouseButtonInput| {
                if !event.state.is_pressed() {
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

                    ctx.remove(node);
                    *active = None;
                }

                if !event.button.is_right() {
                    return;
                }

                let parents = state.parents.try_lock().unwrap();
                for item in parents.iter() {
                    let Some(layout) = ctx.layout(item.node.node().unwrap()) else {
                        continue;
                    };

                    if layout.contains(cursor) {
                        *active = Some(item.node.node().unwrap());

                        let ctx2 = ctx.clone();
                        let closer = Callback::from(move |()| {
                            let Some(state) = ctx2.document().get::<InternalContextMenuState>()
                            else {
                                return;
                            };

                            if let Some(node) = state.active.try_lock().unwrap().take() {
                                ctx2.remove(node);
                            };
                        });

                        let menu_ctx = Container::new()
                            .style(Style {
                                position: Position::Absolute(cursor),
                                ..Default::default()
                            })
                            .mount(&item.node);

                        *active = Some(menu_ctx.node().unwrap());
                        let cb = item.callback.clone();
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
    pub ctx: Context,
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
    parents: Mutex<Vec<ContextMenuItem>>,
    active: Mutex<Option<NodeId>>,
}

#[derive(Debug)]
struct ContextMenuItem {
    node: Context,
    callback: Callback<ContextMenuState>,
    priority: u32,
}
