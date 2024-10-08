use game_ui::runtime::Context;
use game_ui::style::{Background, Growth, Rgba, Style};
use game_ui::widgets::{Container, Widget};

pub struct MainMenu {}

impl Widget for MainMenu {
    fn mount(self, parent: &Context) -> Context {
        Container::new()
            .style(Style {
                growth: Growth::new(1.0, 1.0),
                background: Background::Color(Rgba([128, 128, 128, 90])),
                ..Default::default()
            })
            .mount(parent)
    }
}
