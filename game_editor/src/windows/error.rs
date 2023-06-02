use game_ui::reactive::Scope;
use game_ui::render::style::{Bounds, Direction, Padding, Size, SizeVec2, Style};
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn Error(cx: &Scope, message: &str) -> Scope {
    let style = Style {
        direction: Direction::Column,
        padding: Padding::splat(Size::Pixels(10.0)),
        ..Default::default()
    };

    let root = view! {
        cx,
        <Container style={style}>
        </Container>
    };

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

    view! {
        root,
        <Image image={img.to_rgba8()} style={style}>
        </Image>
    };

    view! {
        root,
        <Text text={message.into()}>
        </Text>
    };

    root
}
