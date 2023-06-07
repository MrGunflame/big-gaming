use game_ui::reactive::Scope;
use game_ui::render::style::Style;
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn CreateRecord(cx: &Scope) -> Scope {
    let style = Style::default();

    let root = view! {
        cx,
        <Container style={style}>
        </Container>
    };

    root
}
