//! Freeform inventory item list

use std::marker::PhantomData;
use std::sync::{mpsc, Arc, Mutex};

use game_core::modules::Modules;
use game_input::mouse::MouseButtonInput;
use game_ui::events::Context;
use game_ui::reactive::{Document, NodeId, Scope};
use game_ui::style::{Bounds, Direction, Growth, Justify, Position, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Container, Text, Widget};
use game_wasm::inventory::{Inventory, InventorySlotId, ItemStack};
use game_wasm::world::RecordReference;
use glam::UVec2;

use super::UiEvent;

pub struct InventoryUi<'a> {
    inventory: &'a Inventory,
    modules: Modules,
    events: mpsc::Sender<UiEvent>,
}

impl<'a> Widget for InventoryUi<'a> {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            bounds: Bounds::from_min(SizeVec2::splat(Size::ZERO)),
            growth: Growth::new(1.0, 1.0),
            ..Default::default()
        }));

        InventoryBox {
            inventory: self.inventory,
            modules: self.modules,
            events: self.events,
        }
        .build(&root);

        root
    }
}

struct InventoryBox<'a> {
    inventory: &'a Inventory,
    modules: Modules,
    events: mpsc::Sender<UiEvent>,
}

impl<'a> Widget for InventoryBox<'a> {
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

        let items = self.inventory.iter();

        ItemList {
            items,
            modules: self.modules,
            events: self.events,
            _marker: PhantomData,
        }
        .build(&align_x);

        align_y
    }
}

struct ItemList<'a, I> {
    items: I,
    modules: Modules,
    events: mpsc::Sender<UiEvent>,
    // FIXME: Can we get rid of this marker?
    _marker: PhantomData<&'a ()>,
}

impl<'a, I> Widget for ItemList<'a, I>
where
    I: Iterator<Item = (InventorySlotId, &'a ItemStack)>,
{
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new());

        let context_menu = Arc::new(Mutex::new(None));

        for (id, stack) in self.items {
            let is_equipped = stack.equipped;

            let events = self.events.clone();

            let root2 = root.clone();
            let context_menu = context_menu.clone();
            let wrapper = root.append(Button::new().on_click(
                move |ctx: Context<MouseButtonInput>| {
                    let mut ctx_menu = context_menu.lock().unwrap();
                    let events = events.clone();

                    if let Some(id) = &*ctx_menu {
                        root2.remove(*id);
                    }

                    *ctx_menu = ContextMenu {
                        position: ctx.cursor.position().as_uvec2(),
                        id,
                        events,
                        is_equipped,
                    }
                    .build(&root2)
                    .id();
                },
            ));

            let module = self.modules.get(stack.item.module).unwrap();
            let record = module.records.get(stack.item.record).unwrap();

            let label = format!("{} ({})", record.name, stack.quantity);
            wrapper.append(Text::new().text(label));
        }

        root
    }
}

struct ContextMenu {
    position: UVec2,
    id: InventorySlotId,
    events: mpsc::Sender<UiEvent>,
    is_equipped: bool,
}

impl Widget for ContextMenu {
    fn build(self, cx: &Scope) -> Scope {
        let root = cx.append(Container::new().style(Style {
            position: Position::Absolute(self.position),
            ..Default::default()
        }));

        if self.is_equipped {
            let events = self.events.clone();
            let button = root.append(Button::new().on_click(move |_ctx| {
                events.send(unequip_event(self.id)).unwrap();
            }));
            button.append(Text::new().text("Unequip".to_string()));
        } else {
            let events = self.events.clone();
            let button = root.append(Button::new().on_click(move |_ctx| {
                events.send(equip_event(self.id)).unwrap();
            }));
            button.append(Text::new().text("Equip".to_string()));
        }

        {
            let events = self.events.clone();
            let button = root.append(Button::new().on_click(move |_ctx| {
                events.send(drop_event(self.id)).unwrap();
            }));
            button.append(Text::new().text("Drop".to_string()));
        }

        root
    }
}

#[derive(Debug)]
pub struct InventoryProxy {
    pub id: NodeId,
}

impl InventoryProxy {
    pub fn new(
        inventory: &Inventory,
        modules: Modules,
        doc: &Document,
        tx: mpsc::Sender<UiEvent>,
    ) -> Self {
        let cx = doc.root_scope();

        let root = cx.append(InventoryUi {
            inventory,
            modules,
            events: tx,
        });

        Self {
            id: root.id().unwrap(),
        }
    }
}

fn equip_event(slot: InventorySlotId) -> UiEvent {
    const ID: RecordReference =
        RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:17");

    let mut data = Vec::new();
    data.extend(slot.into_raw().to_le_bytes());

    UiEvent { id: ID, data }
}

fn unequip_event(slot: InventorySlotId) -> UiEvent {
    const ID: RecordReference =
        RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:18");

    let mut data = Vec::new();
    data.extend(slot.into_raw().to_le_bytes());

    UiEvent { id: ID, data }
}

fn drop_event(slot: InventorySlotId) -> UiEvent {
    const ID: RecordReference =
        RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:19");

    let mut data = Vec::new();
    data.extend(slot.into_raw().to_le_bytes());

    UiEvent { id: ID, data }
}
