use game_ui::reactive::Scope;
use game_ui::render::style::Style;
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn Error(cx: &Scope, message: &str) -> Scope {
    view! {
        cx,
        <Container style={Style::default()}>
            <Text text={message.into()}>
            </Text>
        </Container>
    }
}
