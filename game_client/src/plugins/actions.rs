use ahash::HashMap;
use bevy_app::{App, Plugin};
use bevy_ecs::system::{Res, ResMut, Resource};
use game_common::components::actions::ActionId;
use game_common::events::{ActionEvent, Event, EventQueue};
use game_common::module::ModuleId;
use game_common::record::{RecordId, RecordReference};
use game_core::modules::Modules;
use game_data::record::{Record, RecordBody};
use game_input::hotkeys::{Hotkey, HotkeyCode, HotkeyId, HotkeyReader, Hotkeys, TriggerKind};
use game_input::mouse::MouseButton;
use game_net::snapshot::{Command, EntityAction};

use crate::net::ServerConnection;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveActions::default());
        app.insert_resource(HotkeyMap::default());

        app.add_startup_system(register_actions);
        app.add_system(dispatch_player_action_hotkeys);
    }
}

/// Map hotkeys to actions.
#[derive(Clone, Debug, Default, Resource)]
struct HotkeyMap {
    /// Note that a single hotkey may map to multiple actions.
    hotkeys: HashMap<HotkeyId, Vec<ActionId>>,
}

/// The actions that are currently being listened on.
///
/// What actions are listened on are specified as follows:
/// 1. All actions that are assigned to the actor itself (in form of a Race record) are always
/// enabled.
/// 2. All actions for items that are currently equipped are enabled.
#[derive(Clone, Debug, Default, Resource)]
pub struct ActiveActions {
    hotkeys: HashMap<HotkeyId, Vec<ActionId>>,
}

impl ActiveActions {
    pub fn register(&mut self, hotkeys: &mut Hotkeys, module: ModuleId, record: Record) {
        tracing::info!("registered action for {:?}", record);

        assert!(matches!(record.body, RecordBody::Action(_)));

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

        self.hotkeys
            .entry(id)
            .or_default()
            .push(ActionId(RecordReference {
                module: module,
                record: RecordId(record.id.0),
            }));
    }

    fn get(&self, id: HotkeyId) -> Option<&[ActionId]> {
        self.hotkeys.get(&id).map(|s| s.as_slice())
    }
}

fn dispatch_player_action_hotkeys(
    actions: Res<ActiveActions>,
    mut events: HotkeyReader<Hotkey>,
    mut queue: ResMut<EventQueue>,
    mut conn: ResMut<ServerConnection>,
) {
    let host = conn.host;

    for event in events.iter() {
        let Some(actions) = actions.get(event.id) else {
            continue;
        };

        for action in actions {
            let entity_id = conn.server_entities.get(host).unwrap();

            conn.send(Command::EntityAction(EntityAction {
                id: entity_id,
                action: *action,
            }));

            queue.push(Event::Action(ActionEvent {
                entity: host,
                invoker: host,
                action: *action,
            }));
        }
    }
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
                    record: RecordId(record.id.0),
                }));
        }
    }

    tracing::info!("registered {} action hotkeys", map.hotkeys.len());
}
