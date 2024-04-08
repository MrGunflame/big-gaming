use std::sync::mpsc;

use game_common::collections::string::SmallStr;
use game_common::entity::EntityId;
use game_ui::reactive::{ReadSignal, Scope};
use game_ui::style::{Background, Bounds, Growth, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Container, Text, Widget};

use super::Event;

pub struct Panel {
    pub entities: ReadSignal<Vec<Entity>>,
    pub writer: mpsc::Sender<Event>,
}

impl Widget for Panel {
    fn build(self, cx: &Scope) -> Scope {
        let style = Style {
            background: Background::GRAY,
            growth: Growth::splat(1.0),
            bounds: Bounds::exact(SizeVec2 {
                x: Size::Pixels(300),
                y: Size::Pixels(2000),
            }),
            ..Default::default()
        };

        let root = cx.append(Container::new().style(style));

        let writer = self.writer.clone();
        let create_new_entity = move |ctx| {
            writer.send(Event::Spawn);
        };

        let button = root.append(Button::new().on_click(create_new_entity));
        button.append(Text::new().text("Create".to_owned()));

        root.append(EntityList {
            entities: self.entities,
            writer: self.writer,
        });

        root
    }
}

struct EntityList {
    entities: ReadSignal<Vec<Entity>>,
    writer: mpsc::Sender<Event>,
}

impl Widget for EntityList {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new());

        let list = cx.append(Container::new());

        let mut list_items = Vec::new();
        root.create_effect(move || {
            for id in list_items.drain(..) {
                list.remove(id);
            }

            let entities = self.entities.get();

            for (index, entity) in entities.iter().enumerate() {
                let style = Style {
                    background: if entity.is_selected {
                        Background::YELLOW
                    } else {
                        Background::None
                    },
                    ..Default::default()
                };

                let writer = self.writer.clone();
                let entity_id = entities[index].id;
                let on_click = move |_| {
                    writer.send(Event::SelectEntity(entity_id)).unwrap();
                };

                let button = list.append(Button::new().style(style).on_click(on_click));
                button.append(Text::new().text(entity.name.to_string()));

                list_items.push(button.id().unwrap());
            }
        });

        root
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: EntityId,
    pub name: SmallStr,
    pub is_selected: bool,
}
