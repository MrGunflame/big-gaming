use bevy_app::App;
use bevy_ecs::system::Commands;
use game_ui::render::layout::LayoutTree;
use game_ui::render::style::{Bounds, Direction, Position, Style};
use game_ui::render::{Element, ElementBody, Image, Text};
use game_window::Window;

fn main() {
    let mut app = App::new();

    app.add_plugin(game_ui::render::RenderUiPlugin);
    app.add_startup_system(setup);

    app.run();
}

fn setup(mut cmds: Commands) {
    let mut tree = LayoutTree::new();
    // tree.push(
    //     None,
    //     Element {
    //         body: ElementBody::Text(Text::new("text", 100.0)),
    //         bounds: Bounds {
    //             min: Some(Vec2::new(64.0, 64.0)),
    //             max: None,
    //         },
    //         style: Style::default(),
    //     },
    // );

    let img = image::io::Reader::open("../game_render/img.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

    let root = tree.push(
        None,
        Element {
            body: ElementBody::Container(),
            style: Style {
                direction: Direction::Column,
                ..Default::default()
            },
        },
    );

    let side = tree.push(
        Some(root),
        Element {
            body: ElementBody::Container(),
            style: Style {
                position: Position::default(),
                direction: Direction::Row,
                bounds: Bounds::default(),
            },
        },
    );

    let main = tree.push(
        Some(root),
        Element {
            body: ElementBody::Container(),
            style: Style {
                direction: Direction::Column,
                ..Default::default()
            },
        },
    );

    for _ in 0..5 {
        tree.push(
            Some(side),
            Element {
                body: ElementBody::Text(Text::new("Some record", 100.0)),
                style: Style::default(),
            },
        );
    }

    for _ in 0..4 {
        tree.push(
            Some(main),
            Element {
                body: ElementBody::Image(Image { image: img.clone() }),
                style: Style::default(),
            },
        );
    }

    // tree.push(
    //     None,
    //     Element {
    //         bounds: Bounds {
    //             min: Some(Vec2::new(64.0, 64.0)),
    //             max: None,
    //         },
    //         body: ElementBody::Image(Image { image: img }),
    //         style: Style::default(),
    //     },
    // );

    // let key = tree.push(
    //     None,
    //     Element {
    //         bounds: Bounds::default(),
    //         body: ElementBody::Container(),
    //         style: Style {
    //             direction: Direction::Column,
    //             ..Default::default()
    //         },
    //     },
    // );

    // tree.push(
    //     Some(key),
    //     Element {
    //         bounds: Bounds::default(),
    //         body: ElementBody::Text(Text::new("test", 100.0)),
    //         style: Style::default(),
    //     },
    // );

    // tree.push(
    //     Some(key),
    //     Element {
    //         bounds: Bounds::default(),
    //         body: ElementBody::Text(Text::new("yes", 100.0)),
    //         style: Style::default(),
    //     },
    // );

    cmds.spawn(Window {
        title: "Hello World!".to_owned(),
    })
    .insert(tree.clone());

    cmds.spawn(Window {
        title: "nr2".to_owned(),
    })
    .insert(tree);
}
