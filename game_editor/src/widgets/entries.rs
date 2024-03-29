use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{Scope, WriteSignal};
use game_ui::style::{Background, Direction, Growth, Padding, Size, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};

use super::context_menu::{ContextPanel, State as ContextMenuState};

#[derive(Debug)]
pub struct EntriesData {
    pub keys: Vec<String>,
    pub entries: Vec<Vec<String>>,
    pub add_entry: Option<Callback<Context<MouseButtonInput>>>,
    /// `fn(index: usize)`
    pub edit_entry: Option<Callback<usize>>,
    pub remove_entry: Option<Callback<usize>>,
}

#[derive(Debug)]
struct ContextCallbacks {
    add_entry: Option<Callback<Context<MouseButtonInput>>>,
    edit_entry: Option<Callback<usize>>,
    remove_entry: Option<Callback<usize>>,
}

#[derive(Debug)]
pub struct Entries {
    pub data: EntriesData,
}

impl Widget for Entries {
    fn build(self, cx: &Scope) -> Scope {
        let callbacks = ContextCallbacks {
            add_entry: self.data.add_entry,
            edit_entry: self.data.edit_entry,
            remove_entry: self.data.remove_entry,
        };

        let root = cx.append(
            ContextPanel::new()
                .style(Style {
                    direction: Direction::Column,
                    growth: Growth::splat(1.0),
                    padding: Padding::splat(Size::Pixels(5)),
                    ..Default::default()
                })
                .spawn_menu(spawn_root_ctx_menu(&callbacks)),
        );

        let mut cols = Vec::with_capacity(self.data.keys.len());

        let cell_style = Style {
            padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        };

        for key in &self.data.keys {
            let col = root.append(Container::new().style(cell_style.clone()));
            col.append(Text::new().text(key.to_owned()));

            cols.push(col);
        }

        // Entries are stored in `[Row[Column]]` format, but we need them in
        // `[Column[Row]]` format, which makes layouting easier.

        let mut column_index = 0;
        loop {
            let mut written_cols = 0;

            for (row_index, row) in self.data.entries.iter().enumerate() {
                // Not every row is guaranteed to have the full
                // number of columns. In this case we skip over
                // any missing columns.
                if let Some(column) = row.get(column_index) {
                    let cx = &cols[column_index];

                    let ctx_menu = cx.append(
                        ContextPanel::new().spawn_menu(spawn_ctx_menu(&callbacks, row_index)),
                    );
                    ctx_menu.append(Text::new().text(column.to_owned()));

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
}

fn spawn_root_ctx_menu(
    callbacks: &ContextCallbacks,
) -> Callback<(Scope, WriteSignal<ContextMenuState>)> {
    let add_entry = callbacks.add_entry.clone();

    Callback::from(
        move |(cx, context_menu): (Scope, WriteSignal<ContextMenuState>)| {
            let style = Style {
                background: Background::BLACK,
                padding: Padding::splat(Size::Pixels(2)),
                ..Default::default()
            };

            let root = cx.append(Container::new().style(style));

            if let Some(f) = &add_entry {
                let f = f.clone();

                let button = root.append(Button::new().on_click(move |event| {
                    context_menu.update(|state| state.close());
                    f(event);
                }));
                button.append(Text::new().text("New".to_owned()));
            }
        },
    )
}

fn spawn_ctx_menu(
    callbacks: &ContextCallbacks,
    index: usize,
) -> Callback<(Scope, WriteSignal<ContextMenuState>)> {
    let add_entry = callbacks.add_entry.clone();
    let edit_entry = callbacks.edit_entry.clone();
    let remove_entry = callbacks.remove_entry.clone();

    Callback::from(
        move |(cx, context_menu): (Scope, WriteSignal<ContextMenuState>)| {
            let style = Style {
                background: Background::BLACK,
                padding: Padding::splat(Size::Pixels(5)),
                ..Default::default()
            };

            let root = cx.append(Container::new().style(style));

            if let Some(f) = &add_entry {
                let f = f.clone();
                let context_menu = context_menu.clone();

                let button = root.append(Button::new().on_click(move |event| {
                    context_menu.update(|state| state.close());
                    f(event);
                }));
                button.append(Text::new().text("New".to_owned()));
            }

            if let Some(f) = &edit_entry {
                let f = f.clone();
                let context_menu = context_menu.clone();

                let edit_entry = Box::new(move |_| {
                    context_menu.update(|state| state.close());
                    f(index);
                });

                let button = root.append(Button::new().on_click(edit_entry));
                button.append(Text::new().text("Edit".to_owned()));
            }

            if let Some(f) = &remove_entry {
                let f = f.clone();
                let remove_entry = Box::new(move |_| {
                    context_menu.update(|state| state.close());
                    f(index);
                });

                let button = root.append(Button::new().on_click(remove_entry));
                button.append(Text::new().text("Delete".to_owned()));
            }
        },
    )
}
