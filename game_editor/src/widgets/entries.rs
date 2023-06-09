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
}

#[component]
pub fn Entries(cx: &Scope, data: EntriesData) -> Scope {
    let root = view! {
        cx,
        <ContextMenu spawn_menu={spawn_ctx_menu(data.add_entry)}>
        </ContextMenu>
    };

    let root = view! {
        root,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let mut cols = Vec::with_capacity(data.keys.len());

    for key in data.keys {
        let col = view! {
            root,
            <Container style={Style::default()}>
            </Container>
        };

        view! {
            col,
            <Text text={key.into()}>
            </Text>
        };

        cols.push(col);
    }

    let mut index = 0;
    loop {
        let mut written_cols = 0;

        for row in &data.entries {
            if let Some(col) = row.get(index) {
                let cx = &cols[index];

                view! {
                    cx,
                    <Text text={col.to_owned().into()}>
                    </Text>
                };

                written_cols += 1;
            }
        }

        if written_cols == 0 {
            break;
        }

        index += 1;
    }

    root
}

fn spawn_ctx_menu(
    add_entry: Option<Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>>,
) -> Box<dyn Fn(Scope) + Send + Sync + 'static> {
    let add_entry: Option<Arc<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static>> =
        add_entry.map(|f| Arc::from(f));

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

        view! {
            root,
            <Button on_click={remove_record().into()} style={Style::default()}>
                <Text text={"Delete".into()}>
                </Text>
            </Button>
        };
    })
}

fn remove_record() -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(|_| {
        dbg!("rm");
    })
}
