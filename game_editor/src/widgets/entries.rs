use std::sync::Arc;

use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::Scope;
use game_ui::render::style::{Background, Direction, Padding, Size, Style};
use game_ui::{component, view};

use game_ui::widgets::*;

use crate::widgets::context_menu::*;

pub struct EntriesData {
    pub keys: Vec<String>,
    pub entries: Vec<Vec<String>>,
    pub add_entry: Option<Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>>,
    /// `fn(index: usize)`
    pub edit_entry: Option<Box<dyn Fn(usize) + Send + Sync + 'static>>,
    pub remove_entry: Option<Box<dyn Fn(usize) + Send + Sync + 'static>>,
}

struct ContextCallbacks {
    add_entry: Option<Arc<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>>,
    edit_entry: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    remove_entry: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

#[component]
pub fn Entries(cx: &Scope, data: EntriesData) -> Scope {
    let callbacks = ContextCallbacks {
        add_entry: data.add_entry.map(|f| f.into()),
        edit_entry: data.edit_entry.map(|f| f.into()),
        remove_entry: data.remove_entry.map(|f| f.into()),
    };

    let root = view! {
        cx,
        <ContextMenu spawn_menu={spawn_root_menu(&callbacks)}>
        </ContextMenu>
    };

    let root = view! {
        root,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let mut cols = Vec::with_capacity(data.keys.len());

    for key in &data.keys {
        let col = view! {
            root,
            <Container style={Style::default()}>
            </Container>
        };

        view! {
            col,
            <Text text={key.to_string().into()}>
            </Text>
        };

        cols.push(col);
    }

    // Entries are stored in `[Row[Column]]` format, but we need them in
    // `[Column[Row]]` format, which makes layouting easier.

    let mut column_index = 0;
    loop {
        let mut written_cols = 0;

        for (row_index, row) in data.entries.iter().enumerate() {
            // Not every row is guaranteed to have the full
            // number of columns. In this case we skip over
            // any missing columns.
            if let Some(column) = row.get(column_index) {
                let cx = &cols[column_index];

                view! {
                    cx,
                    <ContextMenu spawn_menu={spawn_ctx_menu(&callbacks, row_index)}>
                        <Text text={column.to_owned().into()}>
                        </Text>
                    </ContextMenu>
                };

                written_cols += 1;
            }
        }

        // End when we have no more columns to write.
        if written_cols == 0 {
            break;
        }

        column_index += 1;
    }

    root
}

fn spawn_root_menu(callbacks: &ContextCallbacks) -> Box<dyn Fn(Scope) + Send + Sync + 'static> {
    let add_entry = callbacks.add_entry.clone();

    Box::new(move |cx| {
        let style = Style {
            background: Background::BLACK,
            padding: Padding::splat(Size::Pixels(2.0)),
            ..Default::default()
        };

        let root = view! {
            cx,
            <Container style={style}>
            </Container>
        };

        if let Some(f) = &add_entry {
            view! {
                root,
                <Button on_click={f.clone().into()} style={Style::default()}>
                    <Text text={"New".into()}>
                    </Text>
                </Button>
            };
        }
    })
}

fn spawn_ctx_menu(
    callbacks: &ContextCallbacks,
    index: usize,
) -> Box<dyn Fn(Scope) + Send + Sync + 'static> {
    let add_entry = callbacks.add_entry.clone();
    let edit_entry = callbacks.edit_entry.clone();
    let remove_entry = callbacks.remove_entry.clone();

    Box::new(move |cx| {
        let style = Style {
            background: Background::BLACK,
            padding: Padding::splat(Size::Pixels(2.0)),
            ..Default::default()
        };

        let root = view! {
            cx,
            <Container style={style}>
            </Container>
        };

        if let Some(f) = &add_entry {
            view! {
                root,
                <Button on_click={f.clone().into()} style={Style::default()}>
                    <Text text={"New".into()}>
                    </Text>
                </Button>
            };
        }

        if let Some(f) = &edit_entry {
            let f = f.clone();
            let edit_entry = Box::new(move |_| {
                f(index);
            });

            view! {
                root,
                <Button on_click={edit_entry.into()} style={Style::default()}>
                    <Text text={"Edit".into()}>
                    </Text>
                </Button>
            };
        }

        if let Some(f) = &remove_entry {
            let f = f.clone();
            let remove_entry = Box::new(move |_| {
                f(index);
            });

            view! {
                root,
                <Button on_click={remove_entry.into()} style={Style::default()}>
                    <Text text={"Delete".into()}>
                    </Text>
                </Button>
            };
        }
    })
}
