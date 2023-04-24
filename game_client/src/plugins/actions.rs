use std::borrow::Cow;

use ahash::HashMap;
use bevy::prelude::{App, MouseButton, Res, ResMut, Resource};
use game_common::components::actions::{Action, ActionId, ActionQueue};
use game_common::components::components::RecordReference;
use game_core::modules::Modules;
use game_data::record::RecordBody;
use game_input::hotkeys::{Hotkey, HotkeyCode, HotkeyId, HotkeyReader, Hotkeys, TriggerKind};

use crate::net::ServerConnection;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionsPlugin;

impl bevy::prelude::Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HotkeyMap::default());
        app.add_startup_system(register_actions);
        app.add_system(handle_player_inputs);
    }
}

/// Map hotkeys to actions.
#[derive(Clone, Debug, Default, Resource)]
struct HotkeyMap {
    /// Note that a single hotkey may map to multiple actions.
    hotkeys: HashMap<HotkeyId, Vec<ActionId>>,
}

fn register_actions(
    mut map: ResMut<HotkeyMap>,
    mut hotkeys: ResMut<Hotkeys>,
    mut modules: Res<Modules>,
) {
    for module in modules.iter() {
        for record in module.records.iter() {
            let RecordBody::Action(action) = &record.body else {
              continue;
            };

            let id = hotkeys.register(Hotkey {
                id: HotkeyId(0),
                name: Cow::Borrowed("test action"),
                default: game_input::hotkeys::Key {
                    trigger: TriggerKind::JUST_PRESSED,
                    code: HotkeyCode::MouseButton {
                        button: MouseButton::Left,
                    },
                },
            });

            map.hotkeys
                .entry(id)
                .or_default()
                .push(ActionId(RecordReference {
                    module: module.id,
                    record: record.id.0,
                }));
        }
    }

    tracing::info!("registered {} action hotkeys", map.hotkeys.len());
}

fn handle_player_inputs(
    conn: Res<ServerConnection>,
    mut map: Res<HotkeyMap>,
    mut events: HotkeyReader<Hotkey>,
    mut queue: ResMut<ActionQueue>,
) {
    let host = conn.host();

    for event in events.iter() {
        let Some(actions) = map.hotkeys.get(&event.id) else {
              continue;
        };

        for action in actions {
            queue.push(Action {
                entity: host,
                id: *action,
                item: None,
            });
        }
    }
}
