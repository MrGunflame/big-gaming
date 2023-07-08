use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::system::Resource;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_effect, create_signal, Scope, WriteSignal};
use game_ui::render::style::Style;
use game_ui::widgets::Text;
use game_ui::{component, view};

use game_ui::widgets::*;
use parking_lot::Mutex;

use crate::backend::{Handle, Task, WriteModule};
use crate::state;
use crate::state::module::EditorModule;

use crate::widgets::entries::*;

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn Modules(
    cx: &Scope,
    modules: state::module::Modules,
    records: state::record::Records,
    handle: Handle,
    queue: SpawnWindowQueue,
    create_modules: CreateModules,
) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let mods = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    let reader = modules.signal(|| {
        let (_, writer) = create_signal(cx, ());
        writer
    });

    let id = Mutex::new(None);
    {
        let modules = modules.clone();
        create_effect(cx, move |world| {
            let _ = reader.get();

            let mut entries = Vec::new();
            for module in modules.iter() {
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
        <Button style={Style::default()} on_click={on_open(queue.clone()).into()}>
            <Text text={"Open".into()}>
            </Text>
        </Button>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_create(queue).into()}>
            <Text text={"Create".into()}>
            </Text>
        </Button>
    };

    view! {
        root,
        <Button style={Style::default()} on_click={on_save(modules, records, handle).into()}>
            <Text text={"Save".into()}>
            </Text>
        </Button>
    };

    root
}

fn on_open(
    queue: SpawnWindowQueue,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        let mut queue = queue.0.write();
        queue.push_back(SpawnWindow::OpenModule);
    })
}

fn on_create(
    queue: SpawnWindowQueue,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        let mut queue = queue.0.write();
        queue.push_back(SpawnWindow::CreateModule);
    })
}

#[derive(Clone, Debug, Default, Resource)]
pub struct CreateModules(pub Arc<Mutex<Option<WriteSignal<Vec<EditorModule>>>>>);

fn on_save(
    modules: state::module::Modules,
    records: state::record::Records,
    handle: Handle,
) -> Box<dyn Fn(Context<MouseButtonInput>) + Send + Sync + 'static> {
    Box::new(move |_| {
        for mut module in modules.iter() {
            if module.path.is_none() {
                module.path = Some(PathBuf::from(module.module.id.to_string()));
            }

            // let records = records
            //     .iter()
            //     .filter(|(id, record)| *id == module.module.id)
            //     .map(|(_, record)| record)
            //     .collect::<Vec<_>>();

            handle.send(Task::WriteModule(WriteModule {
                module: module,
                records: records.clone(),
            }));
        }
    })
}
