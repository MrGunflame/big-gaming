pub mod death;
pub mod debug;
pub mod health;
pub mod inventory;
pub mod main_menu;

use std::sync::mpsc;

use game_ui::reactive::{Context, NodeId};
use game_ui::widgets::Widget;
use game_wasm::world::RecordReference;

use crate::components::base::Health;

use self::death::DealthUi;
use self::debug::{DebugUi, FrametimeGraph, Statistics};
use self::health::HealthUi;

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
        ctx: &Context<()>,
        health: Option<Health>,
        tx: &mpsc::Sender<UiEvent>,
    ) {
        if let Some(id) = self.health {
            ctx.remove(id);
        }

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
    }

    /// Removes all widgets.
    pub fn clear(&mut self, ctx: &Context<()>) {
        if let Some(id) = self.health {
            ctx.remove(id);
        }
    }

    pub fn update_debug_state(&mut self, ctx: &Context<()>, stats: Option<Statistics>) {
        if let Some(id) = self.debug_stats {
            ctx.remove(id);
        }

        if let Some(stats) = stats {
            self.ups.push(stats.ups.last_frametime());
            self.fps.push(stats.fps.last_frametime());
            self.rtt.push(stats.rtt);

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
        }
    }
}

/// Event sent from a UI.
#[derive(Clone, Debug)]
pub struct UiEvent {
    pub id: RecordReference,
    pub data: Vec<u8>,
}
