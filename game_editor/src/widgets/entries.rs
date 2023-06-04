use game_ui::reactive::Scope;
use game_ui::render::style::{Direction, Style};
use game_ui::{component, view};

use game_ui::widgets::*;

#[derive(Clone, Debug)]
pub struct EntriesData {
    pub keys: Vec<String>,
    pub entries: Vec<Vec<String>>,
}

#[component]
pub fn Entries(cx: &Scope, data: EntriesData) -> Scope {
    let root = view! {
        cx,
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
