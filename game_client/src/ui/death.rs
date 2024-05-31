use std::sync::mpsc;

use game_ui::reactive::Scope;
use game_ui::style::{Growth, Style};
use game_ui::widgets::{Button, Container, Text, Widget};
use game_wasm::world::RecordReference;

use super::UiEvent;

pub struct DealthUi {
    pub tx: mpsc::Sender<UiEvent>,
}

impl Widget for DealthUi {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            growth: Growth::new(1.0, 1.0),
            ..Default::default()
        }));

        root.append(Text::new().text("You are ded".to_owned()));
        let respawn = root.append(Button::new().on_click(move |_ctx| {
            self.tx.send(respawn_event()).unwrap();
        }));
        respawn.append(Text::new().text("Respawn".to_owned()));

        root
    }
}

fn respawn_event() -> UiEvent {
    const ID: RecordReference =
        RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:1a");
    UiEvent {
        id: ID,
        data: Vec::new(),
    }
}
