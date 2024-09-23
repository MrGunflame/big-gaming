pub mod death;
pub mod debug;
pub mod health;
pub mod inventory;
pub mod main_menu;
pub mod title_menu;

use std::sync::mpsc;

use game_ui::reactive::{Context, DocumentId, NodeId, Runtime};
use game_ui::widgets::Widget;
use game_wasm::world::RecordReference;

use crate::components::base::Health;

use self::death::DealthUi;
use self::debug::{DebugUi, FrametimeGraph, Statistics};
use self::health::HealthUi;

#[derive(Debug)]
pub struct UiRootContext {
    document: DocumentId,
    widgets: Vec<Context<()>>,
    runtime: Runtime,
}

impl UiRootContext {
    pub fn new(document: DocumentId, runtime: Runtime) -> Self {
        Self {
            document,
            widgets: Vec::new(),
            runtime,
        }
    }

    /// Spawns a new widget.
    pub fn append<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce(&Context<()>) -> U,
    {
        dbg!(self.widgets.len());
        let ctx = self.runtime.root_context(self.document);
        let res = f(&ctx);
        self.widgets.push(ctx);
        res
    }

    pub fn remove(&mut self, id: NodeId) {
        for (index, ctx) in self.widgets.iter().enumerate() {
            if ctx.node() == Some(id) {
                ctx.clone().remove_self();
                self.widgets.remove(index);
                return;
            }
        }
    }

    /// Destroyes all widgets spawned.
    pub fn clear(&mut self) {
        dbg!(&self.widgets.len());
        for ctx in self.widgets.drain(..) {
            ctx.remove_self();
        }
    }
}

// TODO: Move ingame UI into scripts instead of hardcoding
// them here.
#[derive(Debug, Default)]
pub struct UiElements {
    health: Option<NodeId>,
    debug_stats: Option<NodeId>,
    death: Option<NodeId>,
    ups: FrametimeGraph,
    fps: FrametimeGraph,
    rtt: FrametimeGraph,
}

impl UiElements {
    /// Updates the health widget to the given value. Removes the widget if `None` is given.
    pub fn update_health(
        &mut self,
        root: &mut UiRootContext,
        health: Option<Health>,
        tx: &mpsc::Sender<UiEvent>,
    ) {
        if let Some(id) = self.health {
            root.remove(id);
        }

        root.append(|ctx| {
            if let Some(health) = health {
                let id = HealthUi { health }.mount(ctx).node().unwrap();
                self.health = Some(id);

                if let Some(id) = self.death {
                    ctx.remove(id);
                }
            } else {
                if self.death.is_none() {
                    let id = DealthUi { tx: tx.clone() }.mount(ctx).node().unwrap();
                    self.death = Some(id);
                }
            }
        });
    }

    /// Removes all widgets.
    pub fn clear(&mut self, ctx: &Context<()>) {
        if let Some(id) = self.health {
            ctx.remove(id);
        }
    }

    pub fn update_debug_state(&mut self, root: &mut UiRootContext, stats: Option<Statistics>) {
        if let Some(id) = self.debug_stats {
            root.remove(id);
        }

        if let Some(stats) = stats {
            self.ups.push(stats.ups.last_frametime());
            self.fps.push(stats.fps.last_frametime());
            self.rtt.push(stats.rtt);

            root.append(|ctx| {
                let id = DebugUi {
                    stats,
                    ups: self.ups.clone(),
                    fps: self.fps.clone(),
                    rtt: self.rtt.clone(),
                }
                .mount(ctx)
                .node()
                .unwrap();

                self.debug_stats = Some(id);
            });
        }
    }
}

/// Event sent from a UI.
#[derive(Clone, Debug)]
pub struct UiEvent {
    pub id: RecordReference,
    pub data: Vec<u8>,
}
