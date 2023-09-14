use std::path::PathBuf;

use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::Scope;
use game_ui::style::{Growth, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};
use parking_lot::Mutex;

use crate::backend::{Task, WriteModule};
use crate::state::EditorState;

use crate::widgets::entries::*;

use super::SpawnWindow;

pub struct Modules {
    pub state: EditorState,
}

impl Widget for Modules {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new());

        let mods = root.append(Container::new().style(Style {
            growth: Growth::splat(1.0),
            ..Default::default()
        }));

        let reader = self.state.modules.signal(|| {
            let (_, writer) = cx.create_signal(());
            writer
        });

        let id = Mutex::new(None);
        {
            let state = self.state.clone();
            cx.create_effect(move || {
                let _ = reader.get();

                let mut entries = Vec::new();
                for module in state.modules.iter() {
                    entries.push(vec![
                        module.module.id.to_string(),
                        module.module.name.clone(),
                    ]);
                }

                let data = EntriesData {
                    keys: vec!["ID".to_owned(), "Name".to_owned()],
                    entries,
                    add_entry: None,
                    edit_entry: None,
                    remove_entry: None,
                };

                let id = &mut *id.lock();
                match id {
                    Some(id) => {
                        mods.remove(*id);
                    }
                    None => {}
                }

                let cx = mods.append(Entries { data });

                *id = Some(cx.id().unwrap());
            });
        }

        {
            let button = root.append(Button::new().on_click(on_open(self.state.clone())));
            button.append(Text::new().text("Open"));
        }

        {
            let button = root.append(Button::new().on_click(on_create(self.state.clone())));
            button.append(Text::new().text("Create"));
        }

        {
            let button = root.append(Button::new().on_click(on_save(self.state)));
            button.append(Text::new().text("Save"));
        }

        root
    }
}

fn on_open(state: EditorState) -> Callback<Context<MouseButtonInput>> {
    Callback::from(move |_| {
        let _ = state.spawn_windows.send(SpawnWindow::OpenModule);
    })
}

fn on_create(state: EditorState) -> Callback<Context<MouseButtonInput>> {
    Callback::from(move |_| {
        let _ = state.spawn_windows.send(SpawnWindow::CreateModule);
    })
}

fn on_save(state: EditorState) -> Callback<Context<MouseButtonInput>> {
    Callback::from(move |_| {
        for mut module in state.modules.iter() {
            if module.path.is_none() {
                module.path = Some(PathBuf::from(module.module.id.to_string()));
            }

            state.handle.send(Task::WriteModule(WriteModule {
                module: module,
                records: state.records.clone(),
            }));
        }
    })
}
