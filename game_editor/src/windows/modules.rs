use std::sync::Arc;

use bevy_ecs::system::Resource;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{create_effect, create_signal, NodeId, Scope, WriteSignal};
use game_ui::render::style::{Direction, Style};
use game_ui::widgets::Text;
use game_ui::{component, view};

use game_ui::widgets::*;
use parking_lot::Mutex;

use crate::state;
use crate::state::module::EditorModule;

use crate::widgets::entries::*;

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn Modules(
    cx: &Scope,
    modules: state::module::Modules,
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
    create_effect(cx, move |_| {
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
