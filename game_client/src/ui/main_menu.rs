use game_ui::reactive::Context;
use game_ui::style::{Background, Growth, Rgba, Style};
use game_ui::widgets::{Container, Widget};

pub struct MainMenu {}

impl Widget for MainMenu {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        Container::new()
            .style(Style {
                growth: Growth::new(1.0, 1.0),
                background: Background::Color(Rgba([128, 128, 128, 90])),
                ..Default::default()
            })
            .mount(parent)
    }
}
