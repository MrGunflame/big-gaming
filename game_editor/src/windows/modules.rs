use game_ui::render::style::Style;
use game_ui::render::{Element, ElementBody};
use game_ui::widgets::{Context, Text, Widget};

pub fn spawn_modules_window(ctx: &mut Context) {
    let root = Element {
        body: ElementBody::Container(),
        style: Style::default(),
    };

    let root = ctx.tree.push(ctx.parent, root);
    let mut ctx = ctx.child(root);

    Text {
        text: "Modules".to_owned(),
        size: 24.0,
    }
    .create(&mut ctx);

    let main = ctx.tree.push(
        Some(root),
        Element {
            body: ElementBody::Container(),
            style: Style::default(),
        },
    );

    let mut ctx = ctx.child(main);
}
