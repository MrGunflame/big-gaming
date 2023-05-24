//! The file explorer.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use game_ui::reactive::{create_effect, create_signal, Node, Scope};
use game_ui::render::layout::Key;
use game_ui::render::style::{Background, Direction, Growth, Style};
use game_ui::render::{Element, ElementBody};
use game_ui::widgets::{Button, Container, Text, Widget};
use image::Rgba;
use parking_lot::{Mutex, RwLock};

pub fn Explorer(
    cx: &Scope,
    path: PathBuf,
    on_cancel: impl Fn() + Send + Sync + 'static,
    on_open: impl Fn(Vec<Entry>) + Send + Sync + 'static,
) -> Scope {
    let entries = scan(path);

    let (selected_entries, set_selected_entries) = create_signal(cx, entries.clone());

    let root = Container(
        cx,
        Style {
            direction: Direction::Column,
            ..Default::default()
        },
    );

    let side = Container(
        &root,
        Style {
            growth: Growth(Some(1.0)),
            ..Default::default()
        },
    );
    let main = Container(
        &root,
        Style {
            growth: Growth(Some(1.0)),
            ..Default::default()
        },
    );

    for (index, entry) in entries.iter().enumerate() {
        let (select, set_select) = create_signal(cx, false);
        let set_selected_entries = set_selected_entries.clone();

        let but = Button(
            &main,
            move || {
                set_select.update(|v| *v = !*v);
                set_selected_entries.update(|v| v[index].selected ^= true);
            },
            Style::default(),
        );

        let id = but.id().unwrap();
        let cx2 = cx.clone();
        create_effect(cx, move |_| {
            let selected = select.get();
            cx2.set_style(
                id,
                Style {
                    background: if selected {
                        Background::Color(Rgba([255, 0, 0, 255]))
                    } else {
                        Background::None
                    },
                    ..Default::default()
                },
            );
        });

        Text(&but, entry.name.to_string_lossy().to_string());
    }

    let cancel = Button(
        &main,
        move || {
            on_cancel();
        },
        Style::default(),
    );
    let open = Button(
        &main,
        move || {
            let entries = selected_entries
                .get()
                .into_iter()
                .filter(|e| e.selected)
                .collect();
            on_open(entries);
        },
        Style::default(),
    );

    Text(&cancel, "Cancel");
    Text(&open, "Open");

    root
}

pub struct Explorer {
    pub queue: ExplorerQueue,
    pub path: PathBuf,
    pub window: Entity,
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
        });
    }

    entries.sort_by(|a, b| {
        (a.kind as usize)
            .cmp(&(b.kind as usize))
            .then(a.name.cmp(&b.name))
    });

    entries
}

// impl Widget for Explorer {
//     fn create(self, ctx: &mut Context<'_>) -> Key {
//         let window = self.window;
//         let entries = Arc::new(Mutex::new(self.scan()));

//         let root = Element {
//             body: ElementBody::Container(),
//             style: Style {
//                 direction: Direction::Column,
//                 ..Default::default()
//             },
//         };
//         let root = ctx.tree.push(ctx.parent, root);

//         let side = Element {
//             body: ElementBody::Container(),
//             style: Style {
//                 growth: Growth(Some(1.0)),
//                 ..Default::default()
//             },
//         };
//         let side = ctx.tree.push(Some(root), side);

//         let main = ctx.tree.push(
//             Some(root),
//             Element {
//                 body: ElementBody::Container(),
//                 style: Style {
//                     growth: Growth(Some(1.0)),
//                     ..Default::default()
//                 },
//             },
//         );

//         let mut ctx = ctx.child(main);
//         for (index, entry) in entries.lock().iter().enumerate() {
//             let entries = entries.clone();

//             let queue = self.queue.clone();

//             let button = Button {
//                 onclick: Some(Box::new(move |key| {
//                     let mut entry = entries.lock();
//                     entry[index].selected ^= true;

//                     dbg!("x");

//                     let mut queue = queue.0.write();
//                     queue.push_back(Event::Select {
//                         window,
//                         key,
//                         selected: entry[index].selected,
//                     });
//                 })),
//                 style: Style {
//                     direction: Direction::Column,
//                     growth: Growth(None),
//                     ..Default::default()
//                 },
//             }
//             .create(&mut ctx);

//             let mut ctx = ctx.child(button);

//             let text = entry.name.to_string_lossy().to_string();
//             Text { text, size: 24.0 }.create(&mut ctx);

//             let text = file_size(entry.len);
//             Text { text, size: 24.0 }.create(&mut ctx);
//         }

//         let queue = self.queue.clone();
//         LabeledButton {
//             text: "Open".to_owned(),
//             size: 24.0,
//             onclick: Some(Box::new(move |_| {
//                 let entries = entries.lock();

//                 let entries = entries
//                     .iter()
//                     .filter(|entry| entry.selected)
//                     .cloned()
//                     .collect();

//                 let mut queue = queue.0.write();
//                 queue.push_back(Event::Open { entries });
//             })),
//         }
//         .create(&mut ctx);

//         LabeledButton {
//             text: "Cancel".to_owned(),
//             size: 24.0,
//             onclick: Some(Box::new(move |_| {
//                 let mut queue = self.queue.0.write();
//                 queue.push_back(Event::Cancel { window });
//             })),
//         }
//         .create(&mut ctx);

//         root
//     }
// }

#[derive(Clone, Debug)]
pub struct Entry {
    pub kind: EntryKind,
    pub name: OsString,
    pub len: u64,
    pub selected: bool,
    pub path: PathBuf,
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

#[derive(Clone, Debug, Default, Resource)]
pub struct ExplorerQueue(pub Arc<RwLock<VecDeque<Event>>>);

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
