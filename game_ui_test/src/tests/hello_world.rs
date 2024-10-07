use game_ui::runtime::Context;
use game_ui::widgets::{Text, Widget};

pub fn hello_world(ctx: Context) {
    let text = Text::new("Hello World!").size(32.0);
    text.mount(&ctx);
}
