use std::sync::{mpsc, Arc};

use game_common::world::World;
use game_core::modules::Modules;
use game_data::record::RecordKind;
use game_prefab::Prefab;
use game_render::options::MainPassOptions;
use game_render::Renderer;
use game_ui::reactive::Context;
use game_ui::widgets::{Button, Container, Text, Widget};
use game_wasm::world::RecordReference;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use parking_lot::Mutex;

use crate::state::EditorState;
use crate::windows::world::WorldWindowState;

use super::world::panel::Panel;
use super::world::properties::Properties;
use super::world::{Event, SceneState, WorldEvent};
use super::WindowTrait;

pub struct EditWorldWindow {
    state: WorldWindowState,
    ui_state: Arc<Mutex<SceneState>>,
    rx: mpsc::Receiver<Event>,
    new_prefab_rx: mpsc::Receiver<RecordReference>,
    editor_state: EditorState,
}

impl EditWorldWindow {
    pub fn new(ctx: &Context<()>, editor_state: EditorState) -> Self {
        let state = WorldWindowState::new();
        let ui_state: Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, SceneState>> =
            Arc::default();

        let (tx, rx) = mpsc::channel();
        let (new_prefab_tx, new_prefab_rx) = mpsc::channel();

        EditWorld {
            writer: tx,
            state: ui_state.clone(),
            new_prefab_writer: new_prefab_tx,
            editor_state: editor_state.clone(),
        }
        .mount(ctx);

        Self {
            state,
            ui_state,
            editor_state,
            rx,
            new_prefab_rx,
        }
    }
}

impl WindowTrait for EditWorldWindow {
    fn handle_event(&mut self, renderer: &mut Renderer, event: WindowEvent, window_id: WindowId) {
        self.state.handle_event(event, window_id, renderer);
    }

    fn update(
        &mut self,
        world: &mut World,
        renderer: &mut Renderer,
        options: &mut MainPassOptions,
    ) {
        let mut update_entities_panel = false;

        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Spawn => {}
                Event::SelectEntity(entity) => {
                    self.state.toggle_selection(entity);
                    update_entities_panel = true;
                }
                Event::UpdateComponent(id, component) => {}
                Event::DeleteComponent(id) => {}
                Event::SetShadingMode(mode) => {
                    self.state.set_shading_mode(mode);
                }
            }
        }

        while let Ok(id) = self.new_prefab_rx.try_recv() {
            let Some(record) = self.editor_state.records.get(id.module, id.record) else {
                continue;
            };

            let prefab = Prefab::from_bytes(&record.data).unwrap();
            let mut world = World::new();
            prefab.instantiate(&mut world);
            self.state.spawn_world(world);
        }

        while let Some(event) = self.state.pop_event() {
            match event {
                WorldEvent::UpdateTransform(entity, transform) => {}
            }
        }

        if update_entities_panel {
            {
                let entities = self.state.entities();
                self.ui_state.lock().entities = entities;
            }

            let cb = { self.ui_state.lock().entities_changed.clone() };
            cb.call(());
        }

        self.state.update(world, options);
    }
}

struct EditWorld {
    writer: mpsc::Sender<Event>,
    state: Arc<Mutex<SceneState>>,
    new_prefab_writer: mpsc::Sender<RecordReference>,
    editor_state: EditorState,
}

impl Widget for EditWorld {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);
        Properties {
            writer: self.writer.clone(),
        }
        .mount(&root);
        PrefabList {
            editor_state: self.editor_state,
            writer: self.new_prefab_writer,
        }
        .mount(&root);
        Panel {
            state: self.state,
            writer: self.writer,
        }
        .mount(&root);

        root
    }
}

#[derive(Clone, Debug)]
struct PrefabList {
    editor_state: EditorState,
    writer: mpsc::Sender<RecordReference>,
}

impl Widget for PrefabList {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        for (module_id, record) in self.editor_state.records.iter() {
            if record.kind != RecordKind::PREFAB {
                continue;
            }

            let id = RecordReference {
                module: module_id,
                record: record.id,
            };
            let writer = self.writer.clone();
            let button = Button::new()
                .on_click(move |()| {
                    writer.send(id).unwrap();
                })
                .mount(&root);
            Text::new(record.name.clone()).mount(&button);
        }

        root
    }
}
