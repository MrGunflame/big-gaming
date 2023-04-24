use ahash::HashMap;
use bevy::prelude::{App, MouseButton, Res, ResMut, Resource};
use game_common::components::actions::{Action, ActionId};
use game_common::components::components::RecordReference;
use game_common::events::{ActionEvent, EntityEvent, Event, EventQueue};
use game_core::modules::Modules;
use game_data::record::{Record, RecordBody, RecordKind};
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

#[derive(Clone, Debug, Default, Resource)]
struct ActiveActions {
    hotkeys: HashMap<HotkeyId, Vec<ActionId>>,
}

fn register_actions(
    mut map: ResMut<HotkeyMap>,
    mut hotkeys: ResMut<Hotkeys>,
    modules: Res<Modules>,
) {
    for module in modules.iter() {
        for record in module.records.iter() {
            let RecordBody::Action(action) = &record.body else {
              continue;
            };

            let id = hotkeys.register(Hotkey {
                id: HotkeyId(0),
                name: record.name.to_owned().into(),
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
    map: Res<HotkeyMap>,
    mut events: HotkeyReader<Hotkey>,
    mut queue: ResMut<EventQueue>,
) {
    let host = conn.host();

    for event in events.iter() {
        let Some(actions) = map.hotkeys.get(&event.id) else {
            continue;
        };

        for action in actions {
            queue.push(EntityEvent {
                entity: host,
                event: Event::Action(ActionEvent {
                    entity: host,
                    invoker: host,
                    action: *action,
                }),
            });
        }
    }
}
