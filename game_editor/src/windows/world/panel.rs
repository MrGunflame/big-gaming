use std::sync::{mpsc, Arc};

use game_common::collections::string::SmallStr;
use game_common::entity::EntityId;
use game_ui::reactive::Context;
use game_ui::style::{Background, Bounds, Growth, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};
use parking_lot::Mutex;

use super::{Event, SceneState};

pub struct Panel {
    pub state: Arc<Mutex<SceneState>>,
    pub writer: mpsc::Sender<Event>,
}

impl Widget for Panel {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let style = Style {
            background: Background::GRAY,
            growth: Growth::splat(1.0),
            bounds: Bounds::exact(SizeVec2 {
                x: Size::Pixels(300),
                y: Size::Pixels(2000),
            }),
            ..Default::default()
        };

        let root = Container::new().style(style).mount(parent);

        let writer = self.writer.clone();
        let create_new_entity = move |ctx| {
            writer.send(Event::Spawn);
        };

        let button = Button::new().on_click(create_new_entity).mount(&root);
        Text::new("Create").mount(&button);

        EntityList {
            state: self.state,
            writer: self.writer,
        }
        .mount(&root);

        root
    }
}

struct EntityList {
    state: Arc<Mutex<SceneState>>,
    writer: mpsc::Sender<Event>,
}

impl Widget for EntityList {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let parent_ctx = Arc::new(Mutex::new(root.clone()));

        {
            let parent_ctx = parent_ctx.clone();
            let state = self.state.clone();
            let writer = self.writer.clone();
            self.state.lock().entities_changed = Callback::from(move |()| {
                mount_entity_list(&parent_ctx, &state, &writer);
            });
        }

        mount_entity_list(&parent_ctx, &self.state, &self.writer);

        root
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: EntityId,
    pub name: SmallStr,
    pub is_selected: bool,
}

fn mount_entity_list(
    parent: &Arc<Mutex<Context<()>>>,
    state_mux: &Arc<Mutex<SceneState>>,
    writer: &mpsc::Sender<Event>,
) {
    let parent_ctx = parent.lock();
    parent_ctx.clear_children();
    let state = state_mux.lock();

    for (index, entity) in state.entities.iter().enumerate() {
        let style = Style {
            background: if entity.is_selected {
                Background::YELLOW
            } else {
                Background::None
            },
            ..Default::default()
        };

        let id = entity.id;
        let writer = writer.clone();
        let on_click = move |()| {
            writer.send(Event::SelectEntity(id));
        };

        let button = Button::new()
            .style(style)
            .on_click(on_click)
            .mount(&parent_ctx);
        Text::new(entity.name.clone()).mount(&button);
    }
}
