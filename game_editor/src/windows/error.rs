use game_ui::reactive::Scope;
use game_ui::render::style::{Bounds, Direction, Padding, Size, SizeVec2, Style};
use game_ui::widgets::{Container, Image, Text, Widget};

pub struct Error {
    pub message: String,
}

impl Widget for Error {
    fn build(self, cx: &Scope) -> Scope {
        let style = Style {
            direction: Direction::Column,
            padding: Padding::splat(Size::Pixels(10.0)),
            ..Default::default()
        };

        let root = cx.append(Container::new().style(style));

        let img = image::io::Reader::open("/home/robert/Downloads/dialog-error.png")
            .unwrap()
            .decode()
            .unwrap();

        let style = Style {
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(512.0)),
                max: SizeVec2::splat(Size::Pixels(512.0)),
            },
            ..Default::default()
        };

        root.append(Image::new().image(img.to_rgba8()).style(style));
        root.append(Text::new().text(self.message));

        root
    }
}
