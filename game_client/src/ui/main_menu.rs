use game_ui::style::{Background, Growth, Rgba, Style};
use game_ui::widgets::{Container, Widget};

pub struct MainMenu {}

impl Widget for MainMenu {
    fn build(self, cx: &game_ui::reactive::Scope) -> game_ui::reactive::Scope {
        let root = cx.append(Container::new().style(Style {
            growth: Growth::new(1.0, 1.0),
            background: Background::Color(Rgba([128, 128, 128, 90])),
            ..Default::default()
        }));

        root
    }
}
