use game_ui::events::{ElementEventHandlers, EventHandlers};
use game_ui::reactive::{Node, Scope};
use game_ui::render::style::{Background, Direction, Style};
use game_ui::render::{Element, ElementBody};
use game_ui::{component, view};

use game_ui::widgets::*;
use image::Rgba;

use crate::widgets::context_menu::*;

#[derive(Clone, Debug)]
pub struct EntriesData {
    pub keys: Vec<String>,
    pub entries: Vec<Vec<String>>,
}

#[component]
pub fn Entries(cx: &Scope, data: EntriesData) -> Scope {
    let root = view! {
        cx,
        <ContextMenu spawn_menu={spawn_ctx_menu()}>
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

fn spawn_ctx_menu() -> Box<dyn Fn(Scope) + Send + Sync + 'static> {
    Box::new(|cx| {
        let style = Style {
            background: Background::AQUA,
            ..Default::default()
        };

        let root = view! {
            cx,
            <Container style={style}>
            </Container>
        };

        let new_record = view! {
            root,
            <Button on_click={new_record().into()} style={Style::default()}>
                <Text text={"New".into()}>
                </Text>
            </Button>
        };
    })
}

fn new_record() -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(|| {
        dbg!("x");
    })
}
