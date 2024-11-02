use std::sync::{mpsc, Arc, LazyLock};

use game_ui::runtime::reactive::NodeContext;
use game_ui::runtime::Context;
use game_ui::style::{Background, Color, Direction, Style};
use game_ui::widgets::{
    Button, Callback, Container, ContextMenuState, ContextPanel, Svg, SvgData, SvgStyle, Text,
    Widget,
};
use game_wasm::entity::EntityId;
use image::Rgba;
use parking_lot::Mutex;

use super::{Event, SceneState};

const SELECTED_COLOR: Color = Color(Rgba([0x4c, 0x54, 0x59, 0xff]));

static ICON_CUBE: LazyLock<SvgData> = LazyLock::new(|| {
    const BYTES: &[u8] = include_bytes!("../../../../assets/fonts/FontAwesome/svgs/solid/cube.svg");
    SvgData::from_bytes(BYTES).unwrap()
});

#[derive(Clone, Debug)]
pub struct EntityHierarchy {
    pub state: Arc<Mutex<SceneState>>,
    pub writer: mpsc::Sender<Event>,
}

impl Widget for EntityHierarchy {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new().mount(parent);

        let (trigger, set_trigger) = root.runtime().reactive().create_signal(());

        self.state.lock().entities_changed = Callback::from(move |()| {
            set_trigger.set(());
        });

        {
            let root = root.clone();

            parent.runtime().reactive().register_and_schedule_effect(
                move |ctx: &mut NodeContext| {
                    ctx.subscribe(trigger.id());

                    root.clear_children();

                    Text::new("Entities").mount(&root);

                    let spawn = Button::new()
                        .on_click({
                            let writer = self.writer.clone();
                            move |()| {
                                writer.send(Event::Spawn).unwrap();
                            }
                        })
                        .mount(&root);
                    Text::new("Spawn").mount(&spawn);

                    let state = self.state.lock();
                    for entity in &state.entities {
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
                        let writer = self.writer.clone();
                        let on_click = move |()| {
                            writer.send(Event::SelectEntity(id)).unwrap();
                        };

                        let button = Button::new().on_click(on_click).mount(&root);
                        let content = ContextPanel::new()
                            .spawn_menu(spawn_menu(id, self.writer.clone()))
                            .style(style)
                            .mount(&button);

                        Svg::new(ICON_CUBE.clone(), 24, 24)
                            .style(SvgStyle {
                                color: Some(Color::WHITE),
                            })
                            .mount(&content);

                        Text::new(&entity.name).mount(&content);
                    }
                },
            );
        }

        root
    }
}

fn spawn_menu(id: EntityId, writer: mpsc::Sender<Event>) -> impl Into<Callback<ContextMenuState>> {
    move |state: ContextMenuState| {
        for (name, callback) in [
            (
                "New",
                Callback::from({
                    let writer = writer.clone();
                    let closer = state.closer.clone();

                    move |()| {
                        writer.send(Event::Spawn).unwrap();
                        closer.close();
                    }
                }),
            ),
            (
                "Delete",
                Callback::from({
                    let writer = writer.clone();
                    let closer = state.closer.clone();

                    move |()| {
                        writer.send(Event::DespawnEntity(id)).unwrap();
                        closer.close();
                    }
                }),
            ),
        ] {
            let button = Button::new().on_click(callback).mount(&state.ctx);
            Text::new(name).mount(&button);
        }
    }
}
