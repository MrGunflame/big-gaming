use std::sync::{mpsc, Arc};

use ahash::HashMap;
use game_common::components::Transform;
use game_common::world::World;
use game_data::record::{Record, RecordKind};
use game_prefab::Prefab;
use game_render::entities::SceneId;
use game_render::options::MainPassOptions;
use game_render::Renderer;
use game_tracing::trace_span;
use game_ui::runtime::Context;
use game_ui::widgets::{Button, Container, Text, Widget};
use game_wasm::entity::EntityId;
use game_wasm::record::RecordId;
use game_wasm::world::RecordReference;
use game_window::events::WindowEvent;
use game_window::windows::WindowId;
use game_worldgen::{Entity, WorldgenState};
use parking_lot::Mutex;

use crate::state::EditorState;
use crate::windows::world::WorldWindowState;

use super::world::entity_hierarchy::EntityHierarchy;
use super::world::properties::Properties;
use super::world::{Event, SceneState, WorldEvent};
use super::WindowTrait;

pub struct EditWorldWindow {
    state: WorldWindowState,
    ui_state: Arc<Mutex<SceneState>>,
    rx: mpsc::Receiver<Event>,
    new_prefab_rx: mpsc::Receiver<RecordReference>,
    editor_state: EditorState,
    prefabs: HashMap<EntityId, PrefabState>,
    update_entities_panel: bool,
}

impl EditWorldWindow {
    pub fn new(ctx: &Context, editor_state: EditorState) -> Self {
        let mut state = WorldWindowState::new();
        let mut prefabs = HashMap::default();
        let mut update_entities_panel = false;
        for (module, record) in editor_state.records.iter() {
            if record.kind != RecordKind::WORLD_GEN {
                continue;
            }

            let record = match WorldgenState::from_bytes(&record.data) {
                Ok(record) => record,
                Err(err) => {
                    tracing::error!(
                        "failed to decode worldgen state from record {}:{:?}: {:?}",
                        module,
                        record.id,
                        err
                    );
                    continue;
                }
            };

            let is_writable = editor_state
                .modules
                .get(module)
                .unwrap()
                .capabilities
                .write();

            for entity in record.all() {
                let Some(record) = editor_state
                    .records
                    .get(entity.prefab.module, entity.prefab.record)
                else {
                    continue;
                };

                let prefab = match Prefab::from_bytes(&record.data) {
                    Ok(prefab) => prefab,
                    Err(err) => {
                        tracing::error!("invalid prefab data: {}", err);
                        continue;
                    }
                };

                let mut world = World::new();
                let root_entity = prefab.instantiate(&mut world);
                world.insert_typed(root_entity, entity.transform);
                let entity_id = state.spawn_world(world);

                if is_writable {
                    prefabs.insert(
                        entity_id,
                        PrefabState {
                            id: entity.prefab,
                            transform: entity.transform,
                        },
                    );
                    update_entities_panel = true;
                }
            }
        }

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
            prefabs,
            update_entities_panel,
        }
    }

    fn sync_world_state(&self) {
        let _span = trace_span!("EditWorldState::sync_world_state").entered();

        let mut state = WorldgenState::new();
        for prefab in self.prefabs.values() {
            state.insert(Entity {
                prefab: prefab.id,
                transform: prefab.transform,
            });
        }

        // Select a module that is opened as writable.
        // If no module is writable we cannot save the new state.
        // FIXME: What to do if multiple modules are writable?
        let mut module_id = None;
        for module in self.editor_state.modules.iter() {
            if module.capabilities.write() {
                module_id = Some(module.module.id);
            }
        }

        let Some(module) = module_id else {
            return;
        };

        let mut new_record = Record {
            id: RecordId(0),
            kind: RecordKind::WORLD_GEN,
            name: "world_gen".to_owned(),
            description: String::new(),
            data: state.to_bytes(),
        };

        // If there already exists a `WORLD_GEN` record in the module, update
        // that record.
        for (module_id, record) in self.editor_state.records.iter() {
            if module_id != module {
                continue;
            }

            if record.kind == RecordKind::WORLD_GEN {
                new_record.id = record.id;
                self.editor_state.records.update(module, new_record);
                return;
            }
        }

        self.editor_state.records.insert(module, new_record);
    }
}

impl WindowTrait for EditWorldWindow {
    fn handle_event(
        &mut self,
        renderer: &mut Renderer,
        event: WindowEvent,
        window_id: WindowId,
        scene_id: SceneId,
    ) {
        self.state
            .handle_event(event, window_id, renderer, scene_id);
    }

    fn update(&mut self, world: &mut World, options: &mut MainPassOptions) {
        let mut do_sync = false;

        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Spawn => {}
                Event::SelectEntity(entity) => {
                    self.state.toggle_selection(entity);
                    self.update_entities_panel = true;
                }
                Event::UpdateComponent(id, component) => {}
                Event::DeleteComponent(id) => {}
                Event::SetShadingMode(mode) => {
                    self.state.set_shading_mode(mode);
                }
                Event::DespawnEntity(id) => {}
            }
        }

        while let Ok(id) = self.new_prefab_rx.try_recv() {
            let Some(record) = self.editor_state.records.get(id.module, id.record) else {
                continue;
            };

            let prefab = match Prefab::from_bytes(&record.data) {
                Ok(prefab) => prefab,
                Err(err) => {
                    tracing::error!(
                        "record {} ({}) contains invalid prefab data: {}",
                        record.name,
                        record.id,
                        err
                    );
                    continue;
                }
            };

            let mut world = World::new();
            prefab.instantiate(&mut world);
            let entity = self.state.spawn_world(world);
            self.update_entities_panel = true;

            self.prefabs.insert(
                entity,
                PrefabState {
                    id,
                    transform: Transform::default(),
                },
            );
            do_sync = true;
        }

        while let Some(event) = self.state.pop_event() {
            match event {
                WorldEvent::UpdateTransform(entity, transform) => {
                    if let Some(state) = self.prefabs.get_mut(&entity) {
                        state.transform = transform;
                        do_sync = true;
                    }
                }
            }
        }

        if do_sync {
            self.sync_world_state();
        }

        if self.update_entities_panel {
            self.update_entities_panel = false;
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
    fn mount(self, parent: &Context) -> Context {
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
        EntityHierarchy {
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
    fn mount(self, parent: &Context) -> Context {
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

#[derive(Clone, Debug)]
struct PrefabState {
    id: RecordReference,
    transform: Transform,
}
