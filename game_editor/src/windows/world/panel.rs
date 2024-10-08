use std::sync::{mpsc, Arc};

use game_common::collections::string::SmallStr;
use game_common::entity::EntityId;
use game_ui::runtime::Context;
use game_ui::style::{Background, Bounds, Color, Direction, Growth, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Callback, Container, Svg, SvgData, SvgStyle, Text, Widget};
use image::Rgba;
use parking_lot::Mutex;

use super::{Event, SceneState};

const PANEL_COLOR: Color = Color(Rgba([0x16, 0x16, 0x16, 0xff]));
const HEADER_COLOR: Color = Color(Rgba([0x4c, 0x54, 0x59, 0xff]));
const INPUT_COLOR: Color = Color(Rgba([0x2d, 0x31, 0x33, 0xff]));
const SELECTED_COLOR: Color = Color(Rgba([0x4c, 0x54, 0x59, 0xff]));

const ICON_CUBE: &[u8] = include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/cube.svg");

pub struct Panel {
    pub state: Arc<Mutex<SceneState>>,
    pub writer: mpsc::Sender<Event>,
}

impl Widget for Panel {
    fn mount(self, parent: &Context) -> Context {
        let style = Style {
            background: Background::Color(PANEL_COLOR.0),
            growth: Growth::splat(1.0),
            bounds: Bounds::exact(SizeVec2 {
                x: Size::Pixels(200),
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
    fn mount(self, parent: &Context) -> Context {
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
    parent: &Arc<Mutex<Context>>,
    state_mux: &Arc<Mutex<SceneState>>,
    writer: &mpsc::Sender<Event>,
) {
    let parent_ctx = parent.lock();
    parent_ctx.clear_children();
    let state = state_mux.lock();

    let data = SvgData::from_bytes(ICON_CUBE).unwrap();

    for (index, entity) in state.entities.iter().enumerate() {
        let style = Style {
            background: if entity.is_selected {
                Background::Color(SELECTED_COLOR.0)
            } else {
                Background::None
            },
            direction: Direction::Column,
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

        Svg::new(data.clone(), 16, 16)
            .style(SvgStyle {
                color: Some(Color::WHITE),
            })
            .mount(&button);

        Text::new(entity.name.clone()).mount(&button);
    }
}
