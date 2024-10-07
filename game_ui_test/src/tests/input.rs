use game_ui::runtime::Context;
use game_ui::widgets::{Input, Widget};

pub fn input(ctx: Context) {
    let input = Input::new().value("test");
    input.mount(&ctx);
}
