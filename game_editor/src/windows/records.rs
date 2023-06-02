use game_data::record::RecordKind;
use game_ui::reactive::{create_signal, Scope, WriteSignal};
use game_ui::render::style::{Background, Bounds, Growth, Size, SizeVec2, Style};
use game_ui::{component, view};

use game_ui::widgets::*;
use image::Rgba;

const DEFAULT_CATEGORY: RecordKind = RecordKind::Item;

const CATEGORIES: &[RecordKind] = &[
    RecordKind::Item,
    RecordKind::Action,
    RecordKind::Component,
    RecordKind::Object,
];

const SELECTED_COLOR: Background = Background::Color(Rgba([0x04, 0x7d, 0xd3, 0xFF]));

const BACKGROUND_COLOR: [Background; 2] = [
    Background::Color(Rgba([0x50, 0x50, 0x50, 0xFF])),
    Background::Color(Rgba([0x2a, 0x2a, 0x2a, 0xFF])),
];

#[component]
pub fn Records(cx: &Scope) -> Scope {
    let (cat, set_cat) = create_signal(cx, DEFAULT_CATEGORY);

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

    for (index, category) in CATEGORIES.iter().enumerate() {
        let background = BACKGROUND_COLOR[index % 2].clone();

        let style = Style {
            background,
            growth: Growth::x(1.0),
            ..Default::default()
        };

        view! {
            categories,
            <Button style={style} on_click={change_category(*category, set_cat.clone()).into()}>
                <Text text={category_str(*category).into()}>
                </Text>
            </Button>
        };
    }

    root
}

fn change_category(
    category: RecordKind,
    set_cat: WriteSignal<RecordKind>,
) -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(move || {
        set_cat.update(|cat| *cat = category);
    })
}

fn category_str(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Item => "Items",
        RecordKind::Action => "Actions",
        RecordKind::Component => "Components",
        RecordKind::Object => "Object",
    }
}
