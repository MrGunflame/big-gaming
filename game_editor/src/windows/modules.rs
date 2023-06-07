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

use crate::state::module::{self, EditorModule};

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn Modules(cx: &Scope, queue: SpawnWindowQueue, create_modules: CreateModules) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let mut create_modules = create_modules.0.lock();
    let reader = match &*create_modules {
        Some(sig) => sig.subscribe(),
        None => {
            let (reader, writer) = create_signal(&root, vec![]);
            *create_modules = Some(writer);
            reader
        }
    };

    let mods = view! {
        root,
        <Container style={Style::default()}>
        </Container>
    };

    let rows: Mutex<Vec<NodeId>> = Mutex::new(vec![]);
    let cx = mods.clone();
    create_effect(&mods, move |world| {
        // Read to track signal.
        reader.get();

        let mut rows = rows.lock();

        for id in &*rows {
            cx.remove(*id);
        }
        rows.clear();

        let modules = world.resource::<module::Modules>();

        for m in modules.iter() {
            let row = view! {
                cx,
                <Container style={Style { direction: Direction::Column, ..Default::default() }}>
                </Container>
            };

            let id = m.module.id.to_string();
            let name = m.module.name.clone();

            view! {
                row,
                <Text text={id.into()}>
                </Text>
            };

            view! {
                row,
                <Text text={name.into()}>
                </Text>
            };

            rows.push(row.id().unwrap());
        }
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
