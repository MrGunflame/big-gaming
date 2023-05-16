use game_ui::render::layout::Key;
use game_ui::render::style::{Direction, Style};
use game_ui::render::{Element, ElementBody};
use game_ui::widgets::{Context, LabeledButton, Widget};

use crate::windows::{SpawnWindow, SpawnWindowQueue};

pub struct ToolBar {
    pub queue: SpawnWindowQueue,
}

impl Widget for ToolBar {
    fn create(self, ctx: &mut Context<'_>) -> Key {
        let root = ctx.tree.push(
            ctx.parent,
            Element {
                body: ElementBody::Container(),
                style: Style {
                    direction: Direction::Column,
                    ..Default::default()
                },
            },
        );

        let mut ctx = ctx.child(root);

        let queue = self.queue.clone();
        LabeledButton {
            text: "Modules".to_owned(),
            size: 24.0,
            onclick: Some(Box::new(move |_| {
                let mut queue = queue.0.write();
                queue.push_back(SpawnWindow::Modules);
            })),
        }
        .create(&mut ctx);

        LabeledButton {
            text: "Records".to_owned(),
            size: 24.0,
            onclick: Some(Box::new(|_| {
                dbg!("records");
            })),
        }
        .create(&mut ctx);

        let queue = self.queue.clone();
        LabeledButton {
            text: "View".to_owned(),
            size: 24.0,
            onclick: Some(Box::new(move |_| {
                let mut queue = queue.0.write();
                queue.push_back(SpawnWindow::View);
            })),
        }
        .create(&mut ctx);

        root
    }
}
