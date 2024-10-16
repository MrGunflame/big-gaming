use std::sync::{mpsc, Arc, LazyLock};

use game_ui::runtime::reactive::NodeContext;
use game_ui::runtime::Context;
use game_ui::style::{Background, Color, Direction, Style};
use game_ui::widgets::{Button, Callback, Container, Svg, SvgData, SvgStyle, Text, Widget};
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

                        let button = Button::new().style(style).on_click(on_click).mount(&root);

                        Svg::new(ICON_CUBE.clone(), 24, 24)
                            .style(SvgStyle {
                                color: Some(Color::WHITE),
                            })
                            .mount(&button);

                        Text::new(&entity.name).mount(&button);
                    }
                },
            );
        }

        root
    }
}
