//! Freeform inventory item list

use std::sync::{Arc, Mutex};

use game_common::components::inventory::Inventory;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::Scope;
use game_ui::style::{Bounds, Direction, Growth, Justify, Position, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Container, Text, Widget};
use glam::UVec2;

pub struct InventoryUi {
    //inventory: Inventory,
}

impl Widget for InventoryUi {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            bounds: Bounds::from_min(SizeVec2::splat(Size::ZERO)),
            growth: Growth::new(1.0, 1.0),
            ..Default::default()
        }));

        InventoryBox {}.build(&root);

        root
    }
}

struct InventoryBox {}

impl Widget for InventoryBox {
    fn build(self, cx: &Scope) -> Scope {
        let align_y = cx.append(Container::new().style(Style {
            justify: Justify::Center,
            growth: Growth::new(1.0, 1.0),
            ..Default::default()
        }));

        let align_x = align_y.append(Container::new().style(Style {
            justify: Justify::Center,
            growth: Growth::new(1.0, 1.0),
            direction: Direction::Column,
            ..Default::default()
        }));

        ItemList {
            items: vec!["test".into(), "test2".into()],
        }
        .build(&align_x);

        align_y
    }
}

struct ItemList {
    items: Vec<String>,
}

impl Widget for ItemList {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new());

        let context_menu = Arc::new(Mutex::new(None));

        for item in self.items {
            let root2 = root.clone();
            let context_menu = context_menu.clone();
            let wrapper = root.append(Button::new().on_click(
                move |ctx: Context<MouseButtonInput>| {
                    let mut ctx_menu = context_menu.lock().unwrap();

                    if let Some(id) = &*ctx_menu {
                        root2.remove(*id);
                    }

                    *ctx_menu = ContextMenu {
                        position: ctx.cursor.position().as_uvec2(),
                    }
                    .build(&root2)
                    .id();
                },
            ));

            wrapper.append(Text::new().text(item));
        }

        root
    }
}

struct ContextMenu {
    position: UVec2,
}

impl Widget for ContextMenu {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            position: Position::Absolute(self.position),
            ..Default::default()
        }));

        for action in ["Drop"] {
            let button = root.append(Button::new());
            button.append(Text::new().text(action.to_string()));
        }

        root
    }
}
