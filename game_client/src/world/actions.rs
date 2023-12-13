use ahash::HashMap;
use game_common::components::actions::ActionId;
use game_common::module::ModuleId;
use game_common::record::RecordReference;
use game_data::record::Record;
use game_input::hotkeys::{Hotkey, HotkeyId, Hotkeys, Key};
use game_input::keyboard::KeyboardInput;
use game_input::mouse::MouseButtonInput;
use game_tracing::trace_span;

#[derive(Debug)]
pub struct ActiveActions {
    inputs: HashMap<HotkeyId, Vec<ActionId>>,
    hotkeys: Hotkeys,
    actions: HashMap<ActionId, HotkeyId>,
}

impl ActiveActions {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::default(),
            hotkeys: Hotkeys::new(),
            actions: HashMap::default(),
        }
    }

    pub fn register(&mut self, module: ModuleId, record: &Record, key: Key) {
        let _span = trace_span!("ActiveActions::register").entered();
        tracing::info!("registered action for {:?}", record);

        assert!(record.body.as_action().is_some());

        let id = self.hotkeys.register(Hotkey {
            id: HotkeyId(0),
            name: record.name.to_owned().into(),
            default: key,
        });

        self.inputs
            .entry(id)
            .or_default()
            .push(ActionId(RecordReference {
                module,
                record: record.id,
            }));

        self.actions.insert(
            ActionId(RecordReference {
                module,
                record: record.id,
            }),
            id,
        );
    }

    pub fn unregister(&mut self, module: ModuleId, record: &Record) {
        let _span = trace_span!("ActiveActions::unregister").entered();
        tracing::info!("unregistered action for {:?}", record);

        assert!(record.body.as_action().is_some());

        let hotkey = self
            .actions
            .remove(&ActionId(RecordReference {
                module,
                record: record.id,
            }))
            .unwrap();

        let actions = self.inputs.get_mut(&hotkey).unwrap();
        actions.retain(|id| {
            *id != ActionId(RecordReference {
                module,
                record: record.id,
            })
        });

        if actions.is_empty() {
            self.inputs.remove(&hotkey);
            self.hotkeys.unregister(hotkey);
        }
    }

    pub fn clear(&mut self) {
        self.inputs.clear();
        self.hotkeys.clear();
        self.actions.clear();
    }

    pub fn take_events(&mut self) -> Vec<ActionId> {
        let mut events = Vec::new();
        self.hotkeys.send_events(&mut events);

        // Reset the inputs for the next frame.
        self.hotkeys.reset();

        let mut actions = Vec::new();

        for event in events {
            // At least one action should exist for the hotkey, otherwise
            // it wouldn't have been registered.
            let buf = self.inputs.get(&event.id).unwrap();
            actions.extend(buf);
        }

        actions
    }

    pub fn send_keyboard_event(&mut self, event: KeyboardInput) {
        self.hotkeys.send_keyboard_input(event);
    }

    pub fn send_mouse_event(&mut self, event: MouseButtonInput) {
        self.hotkeys.send_mouse_input(event);
    }
}
