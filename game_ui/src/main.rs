use bevy_app::App;
use bevy_ecs::system::Commands;
use game_ui::render::layout::{Bounds, LayoutTree};
use game_ui::render::style::{Direction, Style};
use game_ui::render::{Element, ElementBody, Image, Text};
use game_window::Window;
use glam::Vec2;

fn main() {
    let mut app = App::new();

    app.add_plugin(game_ui::render::RenderUiPlugin);
    app.add_startup_system(setup);

    app.run();
}

fn setup(mut cmds: Commands) {
    let mut tree = LayoutTree::new();
    tree.push(
        None,
        Element {
            body: ElementBody::Text(Text::new("text", 100.0)),
            bounds: Bounds {
                min: Some(Vec2::new(64.0, 64.0)),
                max: None,
            },
            style: Style::default(),
        },
    );

    let img = image::io::Reader::open("../game_render/img.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

    tree.push(
        None,
        Element {
            bounds: Bounds {
                min: Some(Vec2::new(64.0, 64.0)),
                max: None,
            },
            body: ElementBody::Image(Image { image: img }),
            style: Style::default(),
        },
    );

    let key = tree.push(
        None,
        Element {
            bounds: Bounds::default(),
            body: ElementBody::Container(),
            style: Style {
                direction: Direction::Column,
                ..Default::default()
            },
        },
    );

    tree.push(
        Some(key),
        Element {
            bounds: Bounds::default(),
            body: ElementBody::Text(Text::new("test", 100.0)),
            style: Style::default(),
        },
    );

    tree.push(
        Some(key),
        Element {
            bounds: Bounds::default(),
            body: ElementBody::Text(Text::new("testestt", 100.0)),
            style: Style::default(),
        },
    );

    cmds.spawn(Window {
        title: "Hello World!".to_owned(),
    })
    .insert(tree);
}
