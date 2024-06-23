use game_ui::reactive::Context;
use game_ui::style::{Direction, Padding, Size, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};

pub struct Header {
    pub buttons: Vec<ActionButton>,
}

impl Widget for Header {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new()
            .style(Style {
                direction: Direction::Column,
                padding: Padding::splat(Size::Pixels(3)),
                ..Default::default()
            })
            .mount(parent);

        for button in self.buttons {
            let but = Button::new()
                .on_click(button.on_click)
                .style(Style {
                    padding: Padding::splat(Size::Pixels(3)),
                    ..Default::default()
                })
                .mount(&root);
            Text::new(button.label).mount(&but);
        }

        root
    }
}

#[derive(Debug)]
pub struct ActionButton {
    pub label: String,
    pub on_click: Callback<()>,
}
