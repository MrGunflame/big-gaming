//! The file explorer.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::layout::Key;
use game_ui::reactive::{NodeId, ReadSignal, Scope, WriteSignal};
use game_ui::style::{Background, Direction, Growth, Justify, Padding, Size, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};
use game_window::windows::WindowId;
use image::Rgba;
use parking_lot::Mutex;

const BACKGROUND_COLOR: &str = "353535";

const SELECTED_COLOR: &str = "047dd3";

// const SELECTED_COLOR: &str = "2a2a2a";

const TABLE_BACKGROUND_COLOR: [Background; 2] = [
    Background::Color(Rgba([0x50, 0x50, 0x50, 0xFF])),
    Background::Color(Rgba([0x2a, 0x2a, 0x2a, 0xFF])),
];

pub struct Explorer {
    pub on_open: Callback<Vec<Entry>>,
}

impl Widget for Explorer {
    fn build(self, cx: &Scope) -> Scope {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        let (path, set_path) = cx.create_signal(dir);

        let root = cx.append(Container::new().style(Style {
            direction: Direction::Column,
            background: Background::from_hex(BACKGROUND_COLOR).unwrap(),
            ..Default::default()
        }));

        let (selected_entries, set_selected_entries) = cx.create_signal::<Vec<Entry>>(vec![]);

        // let side = view! { root,
        //     <Container style={Style { growth: Growth(None), ..Default::default() }}>
        //     </Container>
        // };

        let main = root.append(Container::new().style(Style {
            growth: Growth::splat(1.0),
            justify: Justify::SpaceBetween,
            ..Default::default()
        }));

        let upper = main.append(Container::new());
        let bottom = main.append(Container::new().style(Style {
            direction: Direction::Column,
            ..Default::default()
        }));

        upper.append(Topbar {
            path: path.clone(),
            directory_up_sig: set_path.clone(),
        });

        let id = Mutex::new(None);
        let cx = upper.clone();
        cx.clone().create_effect(move || {
            let path = path.get();

            let entries = match scan(&path) {
                Ok(entries) => entries,
                Err(err) => {
                    tracing::error!("failed to load {:?}: {}", path, err);

                    // If loading fails (usually for "permission denied") we go
                    // back to the last directory.
                    if path.parent().is_some() {
                        set_path.update(|path| path.pop());
                    }

                    return;
                }
            };

            let mut id = id.lock();
            if let Some(id) = &*id {
                cx.remove(*id);
            }

            let table = upper.append(Container::new().style(Style {
                direction: Direction::Column,
                ..Default::default()
            }));

            *id = Some(table.id().unwrap());
            drop(id);

            let name_col = table.append(Container::new());
            name_col.append(Text::new().text("Name"));

            let signals: Vec<(ReadSignal<_>, WriteSignal<_>)> = (0..entries.len())
                .map(|_| cx.create_signal(false))
                .collect();

            let mut rows: Vec<Vec<NodeId>> = (0..entries.len()).map(|_| vec![]).collect();

            set_selected_entries.update_untracked(|val| *val = entries.clone());

            let on_click = |index: usize, set_selected: WriteSignal<bool>, entry: Entry| {
                let set_selected_entries = set_selected_entries.clone();
                let set_path = set_path.clone();
                move |_: Context<MouseButtonInput>| {
                    set_selected.update(|val| *val ^= true);
                    set_selected_entries.update(|val| val[index].selected ^= true);

                    if entry.kind == EntryKind::Directory {
                        set_path.update(|path| path.push(&entry.name));
                    }
                }
            };

            for (index, entry) in entries.iter().enumerate() {
                let background = TABLE_BACKGROUND_COLOR[index % 2].clone();
                let style = Style {
                    background,
                    growth: Growth::x(1.0),
                    padding: Padding::splat(Size::Pixels(2)),
                    ..Default::default()
                };

                let set_selected = signals[index].1.clone();
                let on_click = on_click(index, set_selected, entry.clone());

                let button = name_col.append(Button::new().style(style).on_click(on_click));
                button.append(Text::new().text(entry.name.to_string_lossy()));

                rows[index].push(button.id().unwrap());
            }

            let date_modified_col = table.append(Container::new());
            date_modified_col.append(Text::new().text("Date Modified"));

            for (index, entry) in entries.iter().enumerate() {
                let background = TABLE_BACKGROUND_COLOR[index % 2].clone();
                let style = Style {
                    background,
                    growth: Growth::x(1.0),
                    padding: Padding::splat(Size::Pixels(2)),
                    ..Default::default()
                };

                let set_selected = signals[index].1.clone();
                let on_click = on_click(index, set_selected, entry.clone());

                let button =
                    date_modified_col.append(Button::new().style(style).on_click(on_click));
                button.append(Text::new().text(format_time(entry.modified)));

                rows[index].push(button.id().unwrap());
            }

            let size_col = table.append(Container::new());
            size_col.append(Text::new().text("Size"));

            for (index, entry) in entries.iter().enumerate() {
                let background = TABLE_BACKGROUND_COLOR[index % 2].clone();
                let style = Style {
                    background,
                    growth: Growth::x(1.0),
                    padding: Padding::splat(Size::Pixels(2)),
                    ..Default::default()
                };

                let set_selected = signals[index].1.clone();
                let on_click = on_click(index, set_selected, entry.clone());

                let button = size_col.append(Button::new().style(style).on_click(on_click));
                button.append(Text::new().text(file_size(entry.len)));

                rows[index].push(button.id().unwrap());
            }

            for (index, (read, _)) in signals.into_iter().enumerate() {
                let row = rows.remove(0);

                let cx2 = cx.clone();
                cx.create_effect(move || {
                    let selected = read.get();

                    let style = if selected {
                        Style {
                            background: Background::from_hex(SELECTED_COLOR).unwrap(),
                            growth: Growth::x(1.0),
                            padding: Padding::splat(Size::Pixels(2)),
                            ..Default::default()
                        }
                    } else {
                        let background = TABLE_BACKGROUND_COLOR[index % 2].clone();
                        Style {
                            background,
                            growth: Growth::x(1.0),
                            padding: Padding::splat(Size::Pixels(2)),
                            ..Default::default()
                        }
                    };

                    for id in &row {
                        cx2.set_style(*id, style.clone());
                    }
                });
            }
        });

        // for (index, entry) in entries.iter().enumerate() {
        //     let (select, set_select) = create_signal(cx, false);
        //     let set_selected_entries = set_selected_entries.clone();

        //     let on_click = move || {
        //         set_select.update(|v| *v = !*v);
        //         set_selected_entries.update(|v| v[index].selected ^= true);
        //     };

        //     let row = view! { upper,
        //         <Button on_click={on_click.into()} style={ Style { direction: Direction::Column, ..Default::default() }}>
        //         </Button>
        //     };

        //     let id = row.id().unwrap();
        //     let cx2 = row.clone();
        //     create_effect(&row, move |_| {
        //         let selected = select.get();
        //         cx2.set_style(
        //             id,
        //             Style {
        //                 background: if selected {
        //                     Background::from_hex(SELECTED_COLOR).unwrap()
        //                 } else {
        //                     Background::None
        //                 },
        //                 direction: Direction::Column,
        //                 padding: Padding::splat(Size::Pixels(2.0)),
        //                 ..Default::default()
        //             },
        //         );
        //     });

        //     view! { row,
        //         <Text text={entry.name.to_string_lossy().to_string().into()}>
        //         </Text>
        //     };

        //     view! {
        //         row,
        //         <Text text={file_size(entry.len).into()}>
        //         </Text>
        //     };

        //     view! {
        //         row,
        //         <Text text={format_time(entry.modified).into()}>
        //         </Text>
        //     };
        // }

        let on_open = move |ctx: Context<MouseButtonInput>| {
            let entries = selected_entries
                .get()
                .into_iter()
                .filter(|e| e.selected)
                .collect();
            (self.on_open)(entries);

            ctx.window.close();
        };

        let on_cancel = move |ctx: Context<MouseButtonInput>| {
            ctx.window.close();
        };

        let cancel = bottom.append(Button::new().on_click(on_cancel));
        cancel.append(Text::new().text("Cancel"));

        let open = bottom.append(Button::new().on_click(on_open));
        open.append(Text::new().text("Open"));

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
    path: ReadSignal<PathBuf>,
    directory_up_sig: WriteSignal<PathBuf>,
}

impl Widget for Topbar {
    fn build(self, cx: &Scope) -> Scope {
        let style = Style {
            direction: Direction::Column,
            ..Default::default()
        };

        let root = cx.append(Container::new().style(style));

        for (text, cb) in [("UP", directory_up(self.directory_up_sig))] {
            let button = root.append(Button::new().on_click(cb));
            button.append(Text::new().text(text));
        }

        let path_box = root.append(Container::new());
        let text_cx = path_box.append(Text::new());

        let id = Mutex::new(text_cx.id().unwrap());
        root.create_effect(move || {
            let text = self.path.get().to_string_lossy().to_string();

            let mut id = id.lock();

            path_box.remove(*id);
            *id = path_box.append(Text::new().text(text)).id().unwrap();
        });

        root
    }
}

fn directory_up(writer: WriteSignal<PathBuf>) -> Callback<Context<MouseButtonInput>> {
    Callback::from(move |_| {
        writer.update(|path| {
            path.pop();
        });
    })
}
