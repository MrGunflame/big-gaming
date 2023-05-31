//! The file explorer.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use chrono::{DateTime, Local};
use game_ui::reactive::{create_effect, create_signal, ReadSignal, Scope, WriteSignal};
use game_ui::render::layout::Key;
use game_ui::render::style::{Background, Direction, Growth, Justify, Padding, Size, Style};
use game_ui::widgets::{Button, ButtonProps, Container, ContainerProps, Text, TextProps};
use game_ui::{component, view};
use image::Rgba;
use parking_lot::RwLock;

const BACKGROUND_COLOR: &str = "353535";

const SELECTED_COLOR: &str = "047dd3";

// const SELECTED_COLOR: &str = "2a2a2a";

#[component]
pub fn Explorer(
    cx: &Scope,
    path: PathBuf,
    on_cancel: Box<dyn Fn() + Send + Sync + 'static>,
    on_open: Box<dyn Fn(Vec<Entry>) + Send + Sync + 'static>,
) -> Scope {
    let entries = scan(path);

    let (selected_entries, set_selected_entries) = create_signal(cx, entries.clone());

    let root = view! { cx,
        <Container style={Style {
            direction: Direction::Column,
            background: Background::from_hex(BACKGROUND_COLOR).unwrap(),
            ..Default::default()
        }}>
        </Container>
    };

    // let side = view! { root,
    //     <Container style={Style { growth: Growth(None), ..Default::default() }}>
    //     </Container>
    // };

    let main = view! { root,
        <Container style={Style { growth: Growth(Some(1.0)), justify: Justify::SpaceBetween, ..Default::default() }}>
        </Container>
    };

    let upper = view! {
        main,
        <Container style={Style::default()}>
        </Container>
    };

    let bottom = view! {
        main,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    for (index, entry) in entries.iter().enumerate() {
        let (select, set_select) = create_signal(cx, false);
        let set_selected_entries = set_selected_entries.clone();

        let on_click = move || {
            set_select.update(|v| *v = !*v);
            set_selected_entries.update(|v| v[index].selected ^= true);
        };

        let row = view! { upper,
            <Button on_click={on_click.into()} style={ Style { direction: Direction::Column, ..Default::default() }}>
            </Button>
        };

        let id = row.id().unwrap();
        let cx2 = row.clone();
        create_effect(&row, move |_| {
            let selected = select.get();
            cx2.set_style(
                id,
                Style {
                    background: if selected {
                        Background::from_hex(SELECTED_COLOR).unwrap()
                    } else {
                        Background::None
                    },
                    direction: Direction::Column,
                    padding: Padding::splat(Size::Pixels(2.0)),
                    ..Default::default()
                },
            );
        });

        view! { row,
            <Text text={entry.name.to_string_lossy().to_string().into()}>
            </Text>
        };

        view! {
            row,
            <Text text={file_size(entry.len).into()}>
            </Text>
        };

        view! {
            row,
            <Text text={format_time(entry.modified).into()}>
            </Text>
        };
    }

    let on_open = move || {
        let entries = selected_entries
            .get()
            .into_iter()
            .filter(|e| e.selected)
            .collect();
        on_open(entries);
    };

    view! { bottom,
        <Button on_click={on_cancel.into()} style={Style::default()}>
            <Text text={"Cancel".into()}></Text>
        </Button>
    };
    view! { bottom,
        <Button on_click={on_open.into()} style={Style::default()}>
            <Text text={"Open".into()}></Text>
        </Button>
    };

    root
}

fn scan(path: PathBuf) -> Vec<Entry> {
    let mut entries = Vec::new();

    let dir = std::fs::read_dir(&path).unwrap();
    for entry in dir {
        let entry = entry.unwrap();

        let meta = entry.metadata().unwrap();

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

    entries
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
        window: Entity,
        key: Key,
        selected: bool,
    },
    Cancel {
        window: Entity,
    },
    Open {
        entries: Vec<Entry>,
    },
}
