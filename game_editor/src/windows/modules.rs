use std::path::PathBuf;

use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_effect, create_signal, Scope};
use game_ui::render::style::{Growth, Style};
use game_ui::widgets::Text;
use game_ui::{component, view};

use game_ui::widgets::*;
use parking_lot::Mutex;

use crate::backend::{Task, WriteModule};
use crate::state::EditorState;

use crate::widgets::entries::*;

use super::SpawnWindow;

#[component]
pub fn Modules(cx: &Scope, state: EditorState) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let mods = view! {
        root,
        <Container style={Style { growth: Growth::splat(1.0), ..Default::default() }}>
        </Container>
    };

    let reader = state.modules.signal(|| {
        let (_, writer) = create_signal(cx, ());
        writer
    });

    let id = Mutex::new(None);
    {
        let state = state.clone();
        create_effect(cx, move || {
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

            let cx = view! {
                mods,
                <Entries data={data}>
                </Entries>
            };

            *id = Some(cx.id().unwrap());
        });
    }

    let open = view! {
        root,
        <Button style={Style::default()} on_click={on_open(state.clone()).into()}>
            <Text text={"Open".into()}>
            </Text>
        </Button>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_create(state.clone()).into()}>
            <Text text={"Create".into()}>
            </Text>
        </Button>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_save(state).into()}>
            <Text text={"Save".into()}>
            </Text>
        </Button>
    };

    root
}

fn on_open(state: EditorState) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        let _ = state.spawn_windows.send(SpawnWindow::OpenModule);
    })
}

fn on_create(state: EditorState) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        let _ = state.spawn_windows.send(SpawnWindow::CreateModule);
    })
}

fn on_save(state: EditorState) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
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
