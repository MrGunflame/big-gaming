use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::Scope;
use game_ui::render::style::{Background, Direction, Padding, Size, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};

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
    add_entry: Option<Callback<Context<MouseButtonInput>>>,
    edit_entry: Option<Callback<usize>>,
    remove_entry: Option<Callback<usize>>,
}

pub struct Entries {
    pub data: EntriesData,
}

impl Widget for Entries {
    fn build(self, cx: &Scope) -> Scope {
        let callbacks = ContextCallbacks {
            add_entry: self.data.add_entry.map(|f| f.into()),
            edit_entry: self.data.edit_entry.map(|f| f.into()),
            remove_entry: self.data.remove_entry.map(|f| f.into()),
        };

        let root = cx.append(ContextMenu {
            spawn_menu: spawn_root_menu(&callbacks),
        });

        let root = root.append(Container::new().style(Style {
            direction: Direction::Column,
            ..Default::default()
        }));

        let mut cols = Vec::with_capacity(self.data.keys.len());

        for key in &self.data.keys {
            let col = cx.append(Container::new());
            col.append(Text::new().text(key));

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

                    let ctx_menu = cx.append(ContextMenu {
                        spawn_menu: spawn_ctx_menu(&callbacks, row_index),
                    });
                    ctx_menu.append(Text::new().text(column));

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

fn spawn_root_menu(callbacks: &ContextCallbacks) -> Callback<Scope> {
    let add_entry = callbacks.add_entry.clone();

    Callback::from(move |cx: Scope| {
        let style = Style {
            background: Background::BLACK,
            padding: Padding::splat(Size::Pixels(2.0)),
            ..Default::default()
        };

        let root = cx.append(Container::new().style(style));

        if let Some(f) = &add_entry {
            let button = root.append(Button::new().on_click(f.clone()));
            button.append(Text::new().text("New"));
        }
    })
}

fn spawn_ctx_menu(callbacks: &ContextCallbacks, index: usize) -> Callback<Scope> {
    let add_entry = callbacks.add_entry.clone();
    let edit_entry = callbacks.edit_entry.clone();
    let remove_entry = callbacks.remove_entry.clone();

    Callback::from(move |cx: Scope| {
        let style = Style {
            background: Background::BLACK,
            padding: Padding::splat(Size::Pixels(2.0)),
            ..Default::default()
        };

        let root = cx.append(Container::new().style(style));

        if let Some(f) = &add_entry {
            let button = root.append(Button::new().on_click(f.clone()));
            button.append(Text::new().text("New"));
        }

        if let Some(f) = &edit_entry {
            let f = f.clone();
            let edit_entry = Box::new(move |_| {
                f(index);
            });

            let button = root.append(Button::new().on_click(edit_entry));
            button.append(Text::new().text("Edit"));
        }

        if let Some(f) = &remove_entry {
            let f = f.clone();
            let remove_entry = Box::new(move |_| {
                f(index);
            });

            let button = root.append(Button::new().on_click(remove_entry));
            button.append(Text::new().text("Delete"));
        }
    })
}
