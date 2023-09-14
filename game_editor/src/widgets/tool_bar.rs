use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::Scope;
use game_ui::style::{Direction, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};

pub struct ToolBar {
    pub buttons: Vec<ActionButton>,
}

impl Widget for ToolBar {
    fn build(self, ctx: &Scope) -> Scope {
        let root = ctx.append(Container::new().style(Style {
            direction: Direction::Column,
            ..Default::default()
        }));

        for button in self.buttons {
            let but = root.append(Button::new().on_click(button.on_click));
            but.append(Text::new().text(button.label));
        }

        root
    }
}

#[derive(Debug)]
pub struct ActionButton {
    pub label: String,
    pub on_click: Callback<Context<MouseButtonInput>>,
}
