//! The file explorer.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Local};
use game_ui::layout::Key;
use game_ui::reactive::Context;
use game_ui::style::{Background, Direction, Growth, Justify, Padding, Size, Style};
use game_ui::widgets::{Button, Callback, Container, Input, Table, Text, Widget};
use game_window::windows::WindowId;
use image::Rgba;
use parking_lot::Mutex;

const BACKGROUND_COLOR: &str = "353535";

const TABLE_BACKGROUND_COLOR: [Background; 2] = [
    Background::Color(Rgba([0x50, 0x50, 0x50, 0xFF])),
    Background::Color(Rgba([0x2a, 0x2a, 0x2a, 0xFF])),
];

const SELECTED_BACKGROUND: Background = Background::Color(Rgba([0x04, 0x7d, 0xd3, 0xff]));

pub struct Explorer {
    pub on_open: Callback<Vec<Entry>>,
}

impl Widget for Explorer {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new()
            .style(Style {
                direction: Direction::Column,
                background: Background::from_hex(BACKGROUND_COLOR).unwrap(),
                ..Default::default()
            })
            .mount(parent);

        // let side = view! { root,
        //     <Container style={Style { growth: Growth(None), ..Default::default() }}>
        //     </Container>
        // };

        let main = Container::new()
            .style(Style {
                growth: Growth::splat(1.0),
                justify: Justify::SpaceBetween,
                ..Default::default()
            })
            .mount(&root);

        let upper = Container::new().mount(&main);
        let bottom = Container::new()
            .style(Style {
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(&main);

        let table_parent = Arc::new(Mutex::new(upper.clone()));
        let state = Arc::new(Mutex::new(State::new()));

        Topbar {
            table_parent: table_parent.clone(),
            state: state.clone(),
        }
        .mount(&upper);

        mount_explorer_table(&table_parent, &state);

        {
            let on_cancel = Callback::from(move |()| {
                // TODO: close window
            });

            let button = Button::new().on_click(on_cancel).mount(&bottom);
            Text::new("Cancel").mount(&button);
        }

        {
            let state = state.clone();
            let on_open = Callback::from(move |()| {
                let state = state.lock();
                let entries = state
                    .entries
                    .iter()
                    .filter(|e| e.selected)
                    .cloned()
                    .collect();

                self.on_open.call(entries);
                // TODO: Window close
            });

            let button = Button::new().on_click(on_open).mount(&bottom);
            Text::new("Open").mount(&button);
        }

        root
    }
}

fn scan(path: impl AsRef<Path>) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();

    let dir = std::fs::read_dir(&path)?;
    for entry in dir {
        let entry = entry?;

        let meta = entry.metadata()?;

        entries.push(Entry {
            kind: if meta.is_file() {
                EntryKind::File
            } else {
                EntryKind::Directory
            },
            name: entry.file_name(),
            len: meta.len(),
            selected: false,
            path: entry.path(),
            modified: meta.modified().ok(),
        });
    }

    entries.sort_by(|a, b| {
        (a.kind as usize)
            .cmp(&(b.kind as usize))
            .then(a.name.cmp(&b.name))
    });

    Ok(entries)
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub kind: EntryKind,
    pub name: OsString,
    pub len: u64,
    pub selected: bool,
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntryKind {
    Directory = 0,
    File = 1,
}

fn file_size(mut bytes: u64) -> String {
    for unit in ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB"] {
        if bytes < 1024 {
            return format!("{} {}", bytes, unit);
        }

        bytes /= 1024;
    }

    format!("{} YiB", bytes)
}

fn format_time(time: Option<SystemTime>) -> String {
    if let Some(time) = time {
        let time = DateTime::<Local>::from(time);

        format!("{}", time.format("%d %b %Y %H %M"))
    } else {
        String::new()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Select {
        window: WindowId,
        key: Key,
        selected: bool,
    },
    Cancel {
        window: WindowId,
    },
    Open {
        entries: Vec<Entry>,
    },
}

struct Topbar {
    table_parent: Arc<Mutex<Context<()>>>,
    state: Arc<Mutex<State>>,
}

impl Widget for Topbar {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let style = Style {
            direction: Direction::Column,
            ..Default::default()
        };

        let root = Container::new().style(style).mount(parent);

        for (text, cb) in [(
            "UP",
            directory_up(self.table_parent.clone(), self.state.clone()),
        )] {
            let button = Button::new().on_click(cb).mount(&root);
            Text::new(text).mount(&button);
        }

        let path_box = Container::new().mount(&root);

        Input::new()
            .value(self.state.lock().path.to_string_lossy())
            .on_change(move |value| {})
            .mount(&path_box);

        root
    }
}

fn directory_up(
    table_parent: Arc<Mutex<Context<()>>>,
    state_mux: Arc<Mutex<State>>,
) -> Callback<()> {
    Callback::from(move |()| {
        {
            let mut state = state_mux.lock();

            // Ignore the event if we are already at the root.
            if !state.path.pop() {
                return;
            }
        }

        mount_explorer_table(&table_parent, &state_mux);
    })
}

fn mount_explorer_table(table_parent: &Arc<Mutex<Context<()>>>, state_mux: &Arc<Mutex<State>>) {
    let mut state = state_mux.lock();

    if state.should_scan {
        tracing::info!("explorer scan");
        let entries = match scan(&state.path) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::error!("failed to load {:?}: {}", state.path, err);

                // If loading fails (usually for "permission denied") we go
                // back to the last directory.
                // if state.path.parent().is_some() {
                //     state.path.pop();
                // }

                return;
            }
        };

        state.should_scan = false;
        state.entries = entries;
    }

    table_parent.lock().clear_children();

    let rows = state
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let background = if entry.selected {
                SELECTED_BACKGROUND
            } else {
                TABLE_BACKGROUND_COLOR[index % 2].clone()
            };
            let style = Style {
                background,
                growth: Growth::x(1.0),
                padding: Padding::splat(Size::Pixels(2)),
                ..Default::default()
            };

            let table_parent = table_parent.clone();
            let state_mux = state_mux.clone();
            let on_click = Callback::from(move |()| {
                {
                    let mut state = state_mux.lock();
                    state.entries[index].selected ^= true;
                }

                mount_explorer_table(&table_parent, &state_mux);
            });

            [
                entry.name.to_string_lossy().to_string(),
                format_time(entry.modified),
                file_size(entry.len),
            ]
            .into_iter()
            .map(|label| LabelButton {
                label,
                style: style.clone(),
                on_click: on_click.clone(),
            })
            .collect()
        })
        .collect();

    let table = Container::new()
        .style(Style {
            direction: Direction::Column,
            ..Default::default()
        })
        .mount(&table_parent.lock());

    Table {
        header: vec![
            Text::new("Name"),
            Text::new("Date Modified"),
            Text::new("Size"),
        ],
        rows,
    }
    .mount(&table);
}

struct State {
    path: PathBuf,
    entries: Vec<Entry>,
    should_scan: bool,
}

impl State {
    pub fn new() -> Self {
        let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        Self {
            path,
            entries: Vec::new(),
            should_scan: true,
        }
    }
}

#[derive(Clone, Debug)]
struct LabelButton {
    label: String,
    style: Style,
    on_click: Callback<()>,
}

impl Widget for LabelButton {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let button = Button::new()
            .style(self.style)
            .on_click(self.on_click)
            .mount(parent);
        Text::new(self.label).mount(&button);
        button
    }
}
