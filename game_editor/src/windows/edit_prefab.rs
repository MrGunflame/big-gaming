use std::sync::{mpsc, Arc};

use game_common::components::components::Components;
use game_common::world::World;
use game_core::modules::Modules;
use game_prefab::Prefab;
use game_ui::reactive::Context;
use game_ui::style::{Direction, Justify, Style};
use game_ui::widgets::{Callback, Container, Widget};
use game_window::windows::WindowId;
use parking_lot::Mutex;

use super::record::EditState;
use super::world::components::ComponentsPanel;
use super::world::panel::Panel;
use super::world::properties::Properties;
use super::world::{Event, OnWorldChangeEvent, SceneState, WorldWindowState};
use super::WindowTrait;

pub fn on_world_change_callback(edit_state: Arc<Mutex<EditState>>) -> Callback<OnWorldChangeEvent> {
    Callback::from(move |event: OnWorldChangeEvent| {
        let mut prefab = Prefab::new();

        for entity in event.entities {
            prefab.add(entity, &event.world);
        }

        let bytes = prefab.to_bytes();
        edit_state.lock().record.data = bytes;
    })
}

pub fn load_prefab(edit_state: &Arc<Mutex<EditState>>) -> World {
    let edit_state = edit_state.lock();

    let prefab = match Prefab::from_bytes(&edit_state.record.data) {
        Ok(prefab) => prefab,
        Err(err) => {
            tracing::warn!("invalid prefab data: {:?}", err);
            return World::default();
        }
    };

    let mut world = World::new();
    prefab.instantiate(&mut world);
    world
}

pub struct EditPrefabWindow {
    state: WorldWindowState,
    rx: mpsc::Receiver<Event>,
    ui_state: Arc<Mutex<SceneState>>,
}

impl EditPrefabWindow {
    pub fn new(ctx: &Context<()>, edit_state: Arc<Mutex<EditState>>, modules: Modules) -> Self {
        let world = load_prefab(&edit_state);

        let mut state = WorldWindowState::new();
        for entity in world.entities() {
            let id = state.spawn();

            for (component_id, component) in world.components(entity).iter() {
                state.insert_component_on_entity(id, component_id, component.clone());
            }
        }
        // handle.spawn(world);

        let (tx, rx) = mpsc::channel();

        let ui_state: Arc<Mutex<SceneState>> = Arc::default();

        PrefabEditor {
            writer: tx,
            modules,
            state: ui_state.clone(),
        }
        .mount(ctx);

        Self {
            state,
            rx,
            ui_state,
        }
    }
}

impl WindowTrait for EditPrefabWindow {
    fn handle_event(
        &mut self,
        renderer: &mut game_render::Renderer,
        event: game_window::events::WindowEvent,
        window_id: WindowId,
    ) {
        self.state.handle_event(event, window_id, renderer);
    }

    fn update(
        &mut self,
        world: &mut World,
        renderer: &mut game_render::Renderer,
        options: &mut game_render::options::MainPassOptions,
    ) {
        let mut update_entities_panel = false;
        let mut update_components_panel = false;

        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Spawn => {
                    self.state.spawn();
                    update_entities_panel = true;
                }
                Event::SelectEntity(entity) => {
                    self.state.toggle_selection(entity);
                    update_entities_panel = true;
                    update_components_panel = true;
                }
                Event::UpdateComponent(id, component) => {
                    self.state.insert_component(id, component);
                }
                Event::DeleteComponent(id) => {
                    self.state.remove_component(id);
                    update_components_panel = true;
                }
                Event::SetShadingMode(mode) => {
                    self.state.set_shading_mode(mode);
                }
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

        if update_components_panel {
            {
                let selected_entities = self
                    .state
                    .entities()
                    .iter()
                    .filter(|v| v.is_selected)
                    .cloned()
                    .collect::<Vec<_>>();

                let components = if selected_entities.is_empty() {
                    Components::new()
                } else {
                    let mut components = self.state.components(selected_entities[0].id);

                    for entity in selected_entities.iter().skip(1) {
                        let other = self.state.components(entity.id);
                        components = components.intersection(&other);
                    }

                    components
                };

                dbg!(&components);

                self.ui_state.lock().components = components;
            }

            let cb = { self.ui_state.lock().components_changed.clone() };
            cb.call(());
        }

        self.state.update(world, options);
    }
}

pub struct PrefabEditor {
    writer: mpsc::Sender<Event>,
    modules: Modules,
    state: Arc<Mutex<SceneState>>,
}

impl Widget for PrefabEditor {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let style = Style {
            direction: Direction::Column,
            justify: Justify::SpaceBetween,
            ..Default::default()
        };

        let root = Container::new().style(style).mount(parent);

        Properties {
            writer: self.writer.clone(),
        }
        .mount(&root);
        Panel {
            writer: self.writer.clone(),
            state: self.state.clone(),
        }
        .mount(&root);
        ComponentsPanel {
            writer: self.writer.clone(),
            modules: self.modules,
            state: self.state,
        }
        .mount(&root);

        root
    }
}
