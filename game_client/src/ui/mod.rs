pub mod death;
pub mod debug;
pub mod health;
pub mod inventory;
pub mod main_menu;

use game_ui::reactive::{NodeId, Scope};
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
}

impl UiElements {
    /// Updates the health widget to the given value. Removes the widget if `None` is given.
    pub fn update_health(&mut self, cx: &mut Scope, health: Option<Health>) {
        if let Some(id) = self.health {
            cx.remove(id);
        }

        if let Some(health) = health {
            let id = HealthUi { health }.build(cx).id().unwrap();
            self.health = Some(id);

            // if let Some(id) = self.death {
            //     cx.remove(id);
            // }
        } else {
            // let id = DealthUi {}.build(cx).id().unwrap();
            // self.death = Some(id);
        }
    }

    /// Removes all widgets.
    pub fn clear(&mut self, cx: &mut Scope) {
        if let Some(id) = self.health {
            cx.remove(id);
        }
    }

    pub fn update_debug_state(&mut self, cx: &mut Scope, stats: Option<Statistics>) {
        if let Some(id) = self.debug_stats {
            cx.remove(id);
        }

        if let Some(stats) = stats {
            self.ups.push(stats.ups.last_frametime());
            self.fps.push(stats.fps.last_frametime());

            let id = DebugUi {
                stats,
                ups: self.ups.clone(),
                fps: self.fps.clone(),
            }
            .build(cx)
            .id()
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
