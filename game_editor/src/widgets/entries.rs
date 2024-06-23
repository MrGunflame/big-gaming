use game_ui::reactive::Context;
use game_ui::style::{Background, Direction, Growth, Padding, Size, Style};
use game_ui::widgets::{Button, Callback, Container, Table, Text, Widget};

use super::context_menu::{ContextMenuState, ContextPanel};

#[derive(Debug)]
pub struct EntriesData {
    pub keys: Vec<String>,
    pub entries: Vec<Vec<String>>,
    pub add_entry: Option<Callback<()>>,
    /// `fn(index: usize)`
    pub edit_entry: Option<Callback<usize>>,
    pub remove_entry: Option<Callback<usize>>,
}

#[derive(Debug)]
struct ContextCallbacks {
    add_entry: Option<Callback<()>>,
    edit_entry: Option<Callback<usize>>,
    remove_entry: Option<Callback<usize>>,
}

#[derive(Debug)]
pub struct Entries {
    pub data: EntriesData,
}

impl Widget for Entries {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let callbacks = ContextCallbacks {
            add_entry: self.data.add_entry,
            edit_entry: self.data.edit_entry,
            remove_entry: self.data.remove_entry,
        };

        let root = Container::new().mount(parent);

        // let root = ContextPanel::new()
        //     .style(Style {
        //         direction: Direction::Column,
        //         growth: Growth::splat(1.0),
        //         padding: Padding::splat(Size::Pixels(5)),
        //         ..Default::default()
        //     })
        //     .spawn_menu(spawn_root_ctx_menu(&callbacks))
        //     .mount(parent);

        let cell_style = Style {
            padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        };

        let header = self
            .data
            .keys
            .into_iter()
            .map(|key| Text::new(key))
            .collect();

        let rows = self
            .data
            .entries
            .into_iter()
            .map(|entry| entry.into_iter().map(|col| Text::new(col)).collect())
            .collect();

        Table { header, rows }.mount(&root);

        root
    }
}

fn spawn_root_ctx_menu(callbacks: &ContextCallbacks) -> Callback<ContextMenuState> {
    let add_entry = callbacks.add_entry.clone();

    Callback::from(move |state: ContextMenuState| {
        let style = Style {
            background: Background::BLACK,
            padding: Padding::splat(Size::Pixels(2)),
            ..Default::default()
        };

        let root = Container::new().style(style).mount(&state.ctx);

        if let Some(f) = &add_entry {
            let f = f.clone();
            let closer = state.closer;

            let button = Button::new()
                .on_click(move |event| {
                    closer.close();
                    f.call(event);
                })
                .mount(&root);
            Text::new("New").mount(&button);
        }
    })
}

fn spawn_ctx_menu(callbacks: &ContextCallbacks, index: usize) -> Callback<ContextMenuState> {
    let add_entry = callbacks.add_entry.clone();
    let edit_entry = callbacks.edit_entry.clone();
    let remove_entry = callbacks.remove_entry.clone();

    Callback::from(move |state: ContextMenuState| {
        let style = Style {
            background: Background::BLACK,
            padding: Padding::splat(Size::Pixels(5)),
            ..Default::default()
        };

        let root = Container::new().style(style).mount(&state.ctx);

        if let Some(f) = &add_entry {
            let f = f.clone();
            let closer = state.closer.clone();

            let button = Button::new()
                .on_click(move |event| {
                    closer.close();
                    f.call(event);
                })
                .mount(&root);
            Text::new("New").mount(&button);
        }

        if let Some(f) = &edit_entry {
            let f = f.clone();
            let closer = state.closer.clone();

            let edit_entry = Box::new(move |_| {
                closer.close();
                f.call(index);
            });

            let button = Button::new().on_click(edit_entry).mount(&root);
            Text::new("Edit").mount(&button);
        }

        if let Some(f) = &remove_entry {
            let f = f.clone();
            let closer = state.closer.clone();

            let remove_entry = Box::new(move |_| {
                closer.close();
                f.call(index);
            });

            let button = Button::new().on_click(remove_entry).mount(&root);
            Text::new("Delete").mount(&button);
        }
    })
}
