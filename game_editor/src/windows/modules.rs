use std::path::PathBuf;
use std::sync::Arc;

use game_ui::reactive::Context;
use game_ui::style::{Growth, Style};
use game_ui::widgets::{Button, Callback, Container, Text, Widget};
use parking_lot::Mutex;

use crate::backend::{Task, WriteModule};
use crate::state::module::EditorModule;
use crate::state::EditorState;

use crate::widgets::entries::*;

use super::SpawnWindow;

pub struct Modules {
    pub state: EditorState,
}

impl Widget for Modules {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        let mods = Container::new()
            .style(Style {
                growth: Growth::splat(1.0),
                ..Default::default()
            })
            .mount(&root);

        let mods_parent = Arc::new(Mutex::new(mods));
        mount_module_table(&mods_parent, self.state.clone());

        let state = self.state.clone();
        self.state.modules.set_on_change(Callback::from(move |()| {
            mount_module_table(&mods_parent, state.clone());
        }));

        {
            let button = Button::new()
                .on_click(on_open(self.state.clone()))
                .mount(&root);
            Text::new("Open").mount(&button);
        }

        {
            let button = Button::new()
                .on_click(on_create(self.state.clone()))
                .mount(&root);
            Text::new("Create").mount(&button);
        }

        {
            let button = Button::new().on_click(on_save(self.state)).mount(&root);
            Text::new("Save").mount(&button);
        }

        root
    }
}

fn on_open(state: EditorState) -> Callback<()> {
    Callback::from(move |_| {
        let _ = state.spawn_windows.send(SpawnWindow::OpenModule);
    })
}

fn on_create(state: EditorState) -> Callback<()> {
    Callback::from(move |_| {
        let _ = state.spawn_windows.send(SpawnWindow::CreateModule);
    })
}

fn on_edit(state: EditorState, entries: Vec<EditorModule>) -> Callback<usize> {
    Callback::from(move |index| {
        let module: &EditorModule = &entries[index];
        let _ = state
            .spawn_windows
            .send(SpawnWindow::EditModule(module.module.id));
    })
}

fn on_remove(state: EditorState, entries: Vec<EditorModule>) -> Callback<usize> {
    Callback::from(move |index| {
        let module: &EditorModule = &entries[index];
        state.modules.remove(module.module.id);
    })
}

fn on_save(state: EditorState) -> Callback<()> {
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

fn mount_module_table(parent: &Arc<Mutex<Context<()>>>, state: EditorState) {
    let ctx = parent.lock();
    ctx.clear_children();

    let mut entry_list = Vec::new();

    let mut entries = Vec::new();
    for module in state.modules.iter() {
        entries.push(vec![
            module.module.id.to_string(),
            module.module.name.clone(),
        ]);

        entry_list.push(module);
    }

    let data = EntriesData {
        keys: vec!["ID".to_owned(), "Name".to_owned(), "Default".to_owned()],
        entries,
        add_entry: Some(on_create(state.clone())),
        edit_entry: Some(on_edit(state.clone(), entry_list.clone())),
        remove_entry: Some(on_remove(state, entry_list)),
    };

    Entries { data }.mount(&ctx);
}
