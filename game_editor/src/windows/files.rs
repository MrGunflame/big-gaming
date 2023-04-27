use std::ffi::OsString;
use std::path::PathBuf;

use bevy::prelude::{Commands, Component, Entity, Plugin, Query, ResMut};
use bevy_egui::egui::CentralPanel;
use bevy_egui::EguiContext;

use crate::backend::{Handle, Task};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilesWindowPlugin;

impl Plugin for FilesWindowPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(render_open_files_windows);
    }
}

#[derive(Clone, Debug, Component)]
pub struct OpenFilesWindow {
    loaded: bool,
    entries: Vec<Entry>,
    path: PathBuf,
}

impl OpenFilesWindow {
    pub fn new() -> Self {
        // Default the current working directory.
        let path = std::env::current_dir().unwrap();

        Self {
            loaded: false,
            entries: vec![],
            path,
        }
    }
}

#[derive(Clone, Debug)]
struct Entry {
    kind: EntryKind,
    name: OsString,
    len: u64,
    selected: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum EntryKind {
    Directory,
    File,
}

fn render_open_files_windows(
    mut commands: Commands,
    mut windows: Query<(Entity, &mut EguiContext, &mut OpenFilesWindow)>,
    mut handle: ResMut<Handle>,
) {
    for (entity, mut ctx, mut state) in &mut windows {
        CentralPanel::default().show(ctx.get_mut(), |ui| {
            ui.heading("Open modules");

            ui.label(state.path.to_string_lossy());

            if !state.loaded {
                state.loaded = true;

                let dir = std::fs::read_dir(&state.path).unwrap();
                for entry in dir {
                    let entry = entry.unwrap();

                    let meta = entry.metadata().unwrap();

                    state.entries.push(Entry {
                        kind: if meta.is_file() {
                            EntryKind::File
                        } else {
                            EntryKind::Directory
                        },
                        name: entry.file_name(),
                        len: meta.len(),
                        selected: false,
                    });
                }
            }

            for entry in &mut state.entries {
                if ui.button(entry.name.to_string_lossy()).clicked() {
                    entry.selected = true;
                }

                ui.label(format!("Selected: {}", entry.selected));
                ui.label(format!("{} bytes", entry.len));
            }

            if ui.button("Open").clicked() {
                let mut shoud_load = true;
                let mut files = vec![];

                for entry in state.entries.iter().filter(|e| e.selected) {
                    match entry.kind {
                        EntryKind::Directory => {
                            let name = entry.name.clone();
                            drop(entry);
                            state.path.push(&name);
                            state.loaded = false;
                            state.entries.clear();

                            // Changed directory, don't load any files.
                            shoud_load = false;
                            break;
                        }
                        EntryKind::File => {
                            let mut path = state.path.clone();
                            path.push(&entry.name);

                            files.push(path);
                        }
                    }
                }

                if shoud_load {
                    commands.entity(entity);

                    for file in files {
                        handle.send(Task::ReadModule(file));
                    }

                    commands.entity(entity).despawn();
                }
            }

            if ui.button("Close").clicked() {
                commands.entity(entity).despawn();
            }
        });
    }
}
