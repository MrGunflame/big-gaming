use game_ui::reactive::Context;
use game_ui::widgets::{Table, Widget};

pub fn table(ctx: Context<()>) {
    let table = Table {
        header: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
        rows: vec![
            vec!["1".to_owned(), "2".to_owned(), "3".to_owned()],
            vec!["4".to_owned(), "5".to_owned(), "6".to_owned()],
        ],
    };
    table.mount(&ctx);
}
