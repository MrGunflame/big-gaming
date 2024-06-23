use game_ui::reactive::Context;
use game_ui::style::{Bounds, Direction, Padding, Size, SizeVec2, Style};
use game_ui::widgets::{Container, Image, Text, Widget};

pub struct Error {
    pub message: String,
}

impl Widget for Error {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let style = Style {
            direction: Direction::Column,
            padding: Padding::splat(Size::Pixels(10)),
            ..Default::default()
        };

        let root = Container::new().style(style).mount(parent);

        let img = image::io::Reader::open("/home/robert/Downloads/dialog-error.png")
            .unwrap()
            .decode()
            .unwrap();

        let style = Style {
            bounds: Bounds {
                min: SizeVec2::splat(Size::Pixels(512)),
                max: SizeVec2::splat(Size::Pixels(512)),
            },
            ..Default::default()
        };

        Image::new().image(img.to_rgba8()).style(style).mount(&root);
        Text::new(self.message).mount(&root);

        root
    }
}
