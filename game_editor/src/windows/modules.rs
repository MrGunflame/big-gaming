use bevy_ecs::world::World;
use game_ui::reactive::Scope;
use game_ui::render::style::Style;
use game_ui::render::{Element, ElementBody};
use game_ui::widgets::{Text, Widget};
use game_ui::{component, view};

use game_ui::widgets::*;

use crate::state::module;

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn Modules(cx: &Scope, modules: &module::Modules, queue: SpawnWindowQueue) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

    for m in modules.iter() {
        let row = view! {
            root,
            <Container style={Style::default()}>
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
