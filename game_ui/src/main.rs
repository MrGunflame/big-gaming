use bevy_app::App;
use bevy_ecs::system::Commands;
use game_ui::render::layout::{Bounds, LayoutTree};
use game_ui::render::{Element, ElementBody, Text};
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
            body: ElementBody::Text(Text::new("text", 64.0)),
            bounds: Bounds {
                min: Some(Vec2::new(64.0, 64.0)),
                max: None,
            },
        },
    );

    cmds.spawn(Window {
        title: "Hello World!".to_owned(),
    })
    .insert(tree);
}
