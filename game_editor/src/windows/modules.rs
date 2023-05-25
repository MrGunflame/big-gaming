use bevy_ecs::world::World;
use game_ui::reactive::Scope;
use game_ui::render::style::Style;
use game_ui::render::{Element, ElementBody};
use game_ui::widgets::{Text, Widget};
use game_ui::{component, view};

use game_ui::widgets::*;

use super::{SpawnWindow, SpawnWindowQueue};

#[component]
pub fn Modules(cx: &Scope, queue: SpawnWindowQueue) -> Scope {
    let root = view! {
        cx,
        <Container style={Style::default()}>
        </Container>
    };

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

// pub fn spawn_modules_window(ctx: &mut Context) {
//     let root = Element {
//         body: ElementBody::Container(),
//         style: Style::default(),
//     };

//     let root = ctx.tree.push(ctx.parent, root);
//     let mut ctx = ctx.child(root);

//     Text {
//         text: "Modules".to_owned(),
//         size: 24.0,
//     }
//     .create(&mut ctx);

//     let main = ctx.tree.push(
//         Some(root),
//         Element {
//             body: ElementBody::Container(),
//             style: Style::default(),
//         },
//     );

//     let mut ctx = ctx.child(main);
// }
