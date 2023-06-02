use game_data::record::{RecordBody, RecordKind};
use game_ui::reactive::{create_signal, Scope, WriteSignal};
use game_ui::render::style::{Background, Bounds, Direction, Growth, Size, SizeVec2, Style};
use game_ui::{component, view};

use game_ui::widgets::*;
use image::Rgba;

use crate::state;

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
pub fn Records(cx: &Scope, records: &state::record::Records) -> Scope {
    let (cat, set_cat) = create_signal(cx, DEFAULT_CATEGORY);

    let root = view! {
        cx,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let categories = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    let main = view! {
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

    for (module_id, record) in records.iter() {
        if record.body.kind() != DEFAULT_CATEGORY {
            continue;
        }

        let mut cols = Vec::new();

        cols.push(record.id.to_string());
        cols.push(record.name.clone());

        match &record.body {
            RecordBody::Item(item) => {
                cols.push(format!("{}g", item.mass.to_grams()));
                cols.push(item.value.to_string());
                cols.push(item.components.len().to_string());
                cols.push(item.actions.len().to_string());
            }
            RecordBody::Action(action) => {}
            RecordBody::Component(component) => {}
            RecordBody::Object(object) => {
                cols.push(object.components.len().to_string());
            }
        }

        let row = view! {
            main,
            <Container style={Style { direction: Direction::Column, ..Default::default() }}>
            </Container>
        };

        for col in cols {
            view! {
                row,
                <Text text={col.into()}>
                </Text>
            };
        }
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
