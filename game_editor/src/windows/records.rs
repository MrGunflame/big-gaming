use game_common::module::ModuleId;
use game_data::record::{Record, RecordBody, RecordKind};
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_effect, create_signal, ReadSignal, Scope, WriteSignal};
use game_ui::render::style::{Background, Direction, Growth, Style};
use game_ui::{component, view};

use game_ui::widgets::*;
use image::Rgba;
use parking_lot::Mutex;

use crate::widgets::entries::*;

use crate::state::{self, EditorState};

use super::SpawnWindow;

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
pub fn Records(cx: &Scope, state: EditorState) -> Scope {
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

    let mut cats = vec![];

    for (index, category) in CATEGORIES.iter().enumerate() {
        let background = BACKGROUND_COLOR[index % 2].clone();

        let style = Style {
            background,
            growth: Growth::x(1.0),
            ..Default::default()
        };

        let cx = view! {
            categories,
            <Button style={style} on_click={change_category(*category, set_cat.clone()).into()}>
                <Text text={category_str(*category).into()}>
                </Text>
            </Button>
        };

        cats.push((*category, cx.id().unwrap()));
    }

    {
        let cat = cat.clone();
        let cx2 = cx.clone();
        create_effect(cx, move || {
            let cat = cat.get();

            for (index, (category, id)) in cats.iter().enumerate() {
                let background = if *category == cat {
                    SELECTED_COLOR.clone()
                } else {
                    BACKGROUND_COLOR[index % 2].clone()
                };

                let style = Style {
                    background,
                    growth: Growth::x(1.0),
                    ..Default::default()
                };

                cx2.set_style(*id, style);
            }
        });
    }

    let reader = state.records.signal(|| {
        let (_, writer) = create_signal(cx, ());
        writer
    });

    let cat_sig = cat;
    let rows = Mutex::new(vec![]);
    let state = state.clone();
    create_effect(cx, move || {
        let _ = reader.get();

        let cat = cat_sig.get();

        let mut rows = rows.lock();

        for id in &*rows {
            main.remove(*id);
        }
        rows.clear();

        let mut keys = vec!["ID".into(), "Name".into()];
        match cat {
            RecordKind::Item => {
                keys.push("Mass".into());
                keys.push("Value".into());
                keys.push("Components".into());
                keys.push("Actions".into());
            }
            RecordKind::Object => {
                keys.push("Components".into());
            }
            _ => (),
        }

        let mut entries = Vec::new();

        let mut entry_records = Vec::new();

        for (module_id, record) in state.records.iter() {
            if record.body.kind() != cat {
                continue;
            }

            let mut row = Vec::new();

            row.push(record.id.to_string());
            row.push(record.name.clone());

            match &record.body {
                RecordBody::Item(item) => {
                    row.push(format!("{}g", item.mass.to_grams()));
                    row.push(item.value.to_string());
                    row.push(item.components.len().to_string());
                    row.push(item.actions.len().to_string());
                }
                RecordBody::Action(action) => {}
                RecordBody::Component(component) => {}
                RecordBody::Object(object) => {
                    row.push(object.components.len().to_string());
                }
                RecordBody::Race(race) => todo!(),
            }

            entries.push(row);

            entry_records.push((module_id, record));
        }

        let entries = EntriesData {
            keys,
            entries,
            add_entry: Some(add_record(state.clone(), cat_sig.clone())),
            edit_entry: Some(edit_record(state.clone(), entry_records.clone())),
            remove_entry: Some(remove_record(entry_records, state.records.clone())),
        };

        let id = view! {
            main,
            <Container style={Style { growth: Growth::splat(1.0), ..Default::default() }}>
                <Entries data={entries}>
                </Entries>
            </Container>
        };

        rows.push(id.id().unwrap());
    });

    root
}

fn change_category(
    category: RecordKind,
    set_cat: WriteSignal<RecordKind>,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        // To prevent unnecessary rerenders only update the category
        // if it actually changed.
        if set_cat.get() != category {
            set_cat.update(|cat| *cat = category);
        }
    })
}

fn category_str(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Item => "Items",
        RecordKind::Action => "Actions",
        RecordKind::Component => "Components",
        RecordKind::Object => "Objects",
        RecordKind::Race => "Race",
    }
}

fn add_record(
    state: EditorState,
    kind: ReadSignal<RecordKind>,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        let kind = kind.get_untracked();

        state.spawn_windows.send(SpawnWindow::CreateRecord(kind));
    })
}

fn edit_record(
    state: EditorState,
    entries: Vec<(ModuleId, Record)>,
) -> Box<dyn Fn(usize) + Send + Sync + 'static> {
    Box::new(move |index| {
        let (module_id, record) = entries[index].clone();

        state
            .spawn_windows
            .send(SpawnWindow::EditRecord(module_id, record));
    })
}

fn remove_record(
    entries: Vec<(ModuleId, Record)>,
    records: state::record::Records,
) -> Box<dyn Fn(usize) + Send + Sync + 'static> {
    Box::new(move |index| {
        let (module_id, record) = entries[index].clone();

        records.remove(module_id, record.id);
    })
}
