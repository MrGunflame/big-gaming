use std::sync::mpsc;

use game_common::record::RecordReference;
use game_data::record::RecordKind;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::Scope;
use game_ui::style::{Background, Growth, Style};
use game_ui::widgets::{Button, Container, Text, Widget};

use crate::state::EditorState;

use super::Event;

pub struct SpawnEntity {
    pub state: EditorState,
    pub writer: mpsc::Sender<Event>,
}

impl Widget for SpawnEntity {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            background: Background::GRAY,
            growth: Growth::splat(1.0),
            ..Default::default()
        }));

        for (light, event) in [
            ("Directional Light", Event::SpawnDirectionalLight),
            ("Point Light", Event::SpawnPointLight),
            ("SpotLight", Event::SpawnSpotLight),
        ] {
            let writer = self.writer.clone();
            let on_spawn = move |ctx: Context<MouseButtonInput>| {
                writer.send(event);
                ctx.window.close();
            };

            let button = root.append(Button::new().on_click(on_spawn));
            button.append(Text::new().text(light));
        }

        for (module_id, record) in self.state.records.iter() {
            // TODO: Support appropriate non-objects.
            if record.kind() != RecordKind::Object {
                continue;
            }

            let writer = self.writer.clone();
            let on_spawn = move |ctx: Context<MouseButtonInput>| {
                writer.send(Event::Spawn(RecordReference {
                    module: module_id,
                    record: record.id,
                }));

                ctx.window.close();
            };

            let button = root.append(Button::new().on_click(on_spawn));
            button.append(Text::new().text(record.name));
        }

        root
    }
}
