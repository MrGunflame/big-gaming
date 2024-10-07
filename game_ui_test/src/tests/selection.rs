use game_ui::runtime::Context;
use game_ui::widgets::{Selection, Widget};

pub fn selection(ctx: Context) {
    let selection = Selection {
        options: vec![
            "Iron".to_owned(),
            "Copper".to_owned(),
            "Tin".to_owned(),
            "Aluminum".to_owned(),
            "Osmium".to_owned(),
        ],
        on_change: (move |index| {
            dbg!(index);
        })
        .into(),
    };
    selection.mount(&ctx);
}
