use std::collections::VecDeque;
use std::sync::Arc;

use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use game_ui::reactive::{create_effect, create_signal, Scope, WriteSignal};
use game_ui::render::style::{Direction, Style};
use game_ui::render::{Element, ElementBody};
use game_ui::widgets::{Text, Widget};
use game_ui::{component, view};

use game_ui::widgets::*;
use parking_lot::Mutex;

use crate::state::module::{self, EditorModule};

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn Modules(
    cx: &Scope,
    modules: &module::Modules,
    queue: SpawnWindowQueue,
    create_modules: CreateModules,
) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    let mut create_modules = create_modules.0.lock();
    let mut reader = match &*create_modules {
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

    let cx = mods.clone();
    create_effect(&mods, move |_| {
        // FIXME: This will take for all windows.
        let modules = reader.with_mut(|m| std::mem::take(m));

        for m in modules {
            let row = view! {
                cx,
                <Container style={Style { direction: Direction::Column, ..Default::default() }}>
                </Container>
            };

            let id = m.module.id.to_string();
            let name = m.module.name;

            dbg!(&id, &name);

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
        }
    });

    for m in modules.iter() {
        let row = view! {
            mods,
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
    }

    let open = view! {
        root,
        <Button style={Style::default()} on_click={on_open(queue).into()}>
            <Text text={"Open".into()}>
            </Text>
        </Button>
    };

    root
}

fn on_open(queue: SpawnWindowQueue) -> Box<dyn Fn() + Send + Sync + 'static> {
    Box::new(move || {
        let mut queue = queue.0.write();
        queue.push_back(SpawnWindow::OpenModule);
    })
}

#[derive(Clone, Debug, Default, Resource)]
pub struct CreateModules(pub Arc<Mutex<Option<WriteSignal<Vec<EditorModule>>>>>);
