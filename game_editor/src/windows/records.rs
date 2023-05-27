use game_data::record::RecordKind;
use game_ui::reactive::Scope;
use game_ui::render::style::Style;
use game_ui::{component, view};

use game_ui::widgets::*;

const CATEGORIES: &[RecordKind] = &[
    RecordKind::Item,
    RecordKind::Action,
    RecordKind::Component,
    RecordKind::Object,
];

#[component]
pub fn Records(cx: &Scope) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let categories = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    for category in CATEGORIES {
        view! {
            categories,
            <Text text={category_str(*category).into()}>
            </Text>
        };
    }

    root
}

fn category_str(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Item => "Items",
        RecordKind::Action => "Actions",
        RecordKind::Component => "Components",
        RecordKind::Object => "Object",
    }
}
