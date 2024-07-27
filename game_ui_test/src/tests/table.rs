use game_ui::reactive::Context;
use game_ui::widgets::{Table, Text, Widget};

pub fn table(ctx: Context<()>) {
    let table = Table::new(
        vec![Text::new("A"), Text::new("B"), Text::new("C")],
        vec![
            vec![Text::new("1"), Text::new("2"), Text::new("3")],
            vec![Text::new("4"), Text::new("5"), Text::new("6")],
        ],
    );
    table.mount(&ctx);
}
