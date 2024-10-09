use game_ui::runtime::Context;
use game_ui::style::{Color, Direction, Justify, Style};
use game_ui::widgets::{Button, Container, Svg, SvgData, SvgStyle, Text, Widget};

const ICON_ERROR: &[u8] =
    include_bytes!("../../../assets/fonts/FontAwesome/svgs/solid/circle-exclamation.svg");

pub struct Error {
    pub message: String,
}

impl Widget for Error {
    fn mount(self, parent: &Context) -> Context {
        let root = Container::new()
            .style(Style {
                ..Default::default()
            })
            .mount(parent);

        Text::new("Error").size(32.0).mount(&root);

        let row = Container::new()
            .style(Style {
                direction: Direction::Column,
                justify: Justify::Center,
                ..Default::default()
            })
            .mount(&root);

        Svg::new(SvgData::from_bytes(ICON_ERROR).unwrap(), 128, 128)
            .style(SvgStyle {
                color: Some(Color::RED),
            })
            .mount(&row);
        Text::new(self.message).mount(&row);

        let close_button = Button::new().mount(&root);
        Text::new("Ok").mount(&close_button);

        root
    }
}
