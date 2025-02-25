use game_ui::runtime::Context;
use game_ui::style::{Background, Border, Color, Direction, Growth, Padding, Size, Style};
use game_ui::widgets::{
    Button, Callback, Container, ContextMenuState, ContextPanel, Table, TableStyle, Text, Widget,
};
use image::Rgba;

const ROOT_CTX_PRIORITY: u32 = 0;
const CELL_CTX_PRIORITY: u32 = 1;

const CONTEXT_MENU_BACKGROUND: Color = Color(Rgba([0x42, 0x41, 0x4d, 0xff]));

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
    fn mount(self, parent: &Context) -> Context {
        let callbacks = ContextCallbacks {
            add_entry: self.data.add_entry,
            edit_entry: self.data.edit_entry,
            remove_entry: self.data.remove_entry,
        };

        let root = ContextPanel::new()
            .style(Style {
                direction: Direction::Column,
                growth: Growth::splat(1.0),
                padding: Padding::splat(Size::Pixels(5)),
                ..Default::default()
            })
            .spawn_menu(spawn_root_ctx_menu(&callbacks))
            .priority(ROOT_CTX_PRIORITY)
            .mount(parent);

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
            .enumerate()
            .map(|(index, entry)| {
                entry
                    .into_iter()
                    .map(|col| TableCell {
                        callbacks: &callbacks,
                        index,
                        label: col,
                    })
                    .collect()
            })
            .collect();

        Table::new(header, rows)
            .style(TableStyle {
                cell_border: Border::splat(Size::Pixels(1), Color::WHITE),
            })
            .mount(&root);

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
            padding: Padding::splat(Size::Pixels(5)),
            background: Background::Color(CONTEXT_MENU_BACKGROUND.0),
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

#[derive(Clone, Debug)]
struct TableCell<'a> {
    callbacks: &'a ContextCallbacks,
    index: usize,
    label: String,
}

impl<'a> Widget for TableCell<'a> {
    fn mount(self, parent: &Context) -> Context {
        let context_menu = ContextPanel::new()
            .spawn_menu(spawn_ctx_menu(&self.callbacks, self.index))
            .priority(CELL_CTX_PRIORITY)
            .style(Style {
                growth: Growth::x(1.0),
                padding: Padding::splat(Size::Pixels(1)),
                ..Default::default()
            })
            .mount(parent);
        Text::new(self.label).mount(&context_menu);
        context_menu
    }
}
