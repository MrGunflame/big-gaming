use game_ui::reactive::Scope;
use game_ui::render::style::{Direction, Padding, Size, Style};
use game_ui::{component, view};

use game_ui::widgets::*;

#[component]
pub fn ToolBar(cx: &Scope, buttons: Vec<ActionButton>) -> Scope {
    let root = view! {
        cx,
        <Container style={Style { direction: Direction::Column, ..Default::default() }}>
        </Container>
    };

    let style = Style {
        padding: Padding::splat(Size::Pixels(2.0)),
        ..Default::default()
    };

    for button in buttons {
        view! {
            root,
            <Button on_click={button.on_click.into()} style={style.clone()}>
                <Text text={button.label.into()}>
                </Text>
            </Button>
        };
    }

    root
}

pub struct ActionButton {
    pub label: String,
    pub on_click: Box<dyn Fn() + Send + Sync + 'static>,
}

// impl Widget for ToolBar {
//     fn create(self, ctx: &mut Context<'_>) -> Key {
//         let root = ctx.tree.push(
//             ctx.parent,
//             Element {
//                 body: ElementBody::Container(),
//                 style: Style {
//                     direction: Direction::Column,
//                     ..Default::default()
//                 },
//             },
//         );

//         let mut ctx = ctx.child(root);

//         let queue = self.queue.clone();
//         LabeledButton {
//             text: "Modules".to_owned(),
//             size: 24.0,
//             onclick: Some(Box::new(move |_| {
//                 let mut queue = queue.0.write();
//                 queue.push_back(SpawnWindow::Modules);
//             })),
//         }
//         .create(&mut ctx);

//         LabeledButton {
//             text: "Records".to_owned(),
//             size: 24.0,
//             onclick: Some(Box::new(|_| {
//                 dbg!("records");
//             })),
//         }
//         .create(&mut ctx);

//         let queue = self.queue.clone();
//         LabeledButton {
//             text: "View".to_owned(),
//             size: 24.0,
//             onclick: Some(Box::new(move |_| {
//                 let mut queue = queue.0.write();
//                 queue.push_back(SpawnWindow::View);
//             })),
//         }
//         .create(&mut ctx);

//         root
//     }
// }
