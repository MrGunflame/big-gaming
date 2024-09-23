//! Freeform inventory item list

use std::sync::mpsc;

use game_core::modules::Modules;
use game_ui::reactive::{Context, NodeId};
use game_ui::style::{Bounds, Direction, Growth, Justify, Size, SizeVec2, Style};
use game_ui::widgets::{Button, Callback, Container, ContextMenuState, ContextPanel, Text, Widget};
use game_wasm::inventory::{Inventory, InventorySlotId, ItemStack};
use game_wasm::world::RecordReference;

use super::{UiEvent, UiRootContext};

const EVENT_EQUIP: RecordReference =
    RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:17");
const EVENT_UNEQUIP: RecordReference =
    RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:18");
const EVENT_DROP: RecordReference =
    RecordReference::from_str_const("c626b9b0ab1940aba6932ea7726d0175:19");

pub struct InventoryUi<'a> {
    inventory: &'a Inventory,
    modules: Modules,
    events: mpsc::Sender<UiEvent>,
}

impl<'a> Widget for InventoryUi<'a> {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new()
            .style(Style {
                bounds: Bounds::from_min(SizeVec2::splat(Size::ZERO)),
                growth: Growth::new(1.0, 1.0),
                ..Default::default()
            })
            .mount(parent);

        InventoryBox {
            inventory: self.inventory,
            modules: self.modules,
            events: self.events,
        }
        .mount(&root);

        root
    }
}

struct InventoryBox<'a> {
    inventory: &'a Inventory,
    modules: Modules,
    events: mpsc::Sender<UiEvent>,
}

impl<'a> Widget for InventoryBox<'a> {
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let align_y = Container::new()
            .style(Style {
                justify: Justify::Center,
                growth: Growth::new(1.0, 1.0),
                ..Default::default()
            })
            .mount(parent);

        let align_x = Container::new()
            .style(Style {
                justify: Justify::Center,
                growth: Growth::new(1.0, 1.0),
                direction: Direction::Column,
                ..Default::default()
            })
            .mount(&align_y);

        let items = self.inventory.iter();

        ItemList {
            items,
            modules: self.modules,
            events: self.events,
        }
        .mount(&align_x);

        align_y
    }
}

struct ItemList<I> {
    items: I,
    modules: Modules,
    events: mpsc::Sender<UiEvent>,
}

impl<'a, I> Widget for ItemList<I>
where
    I: Iterator<Item = (InventorySlotId, &'a ItemStack)>,
{
    fn mount<T>(self, parent: &Context<T>) -> Context<()> {
        let root = Container::new().mount(parent);

        for (id, stack) in self.items {
            let Some(module) = self.modules.get(stack.item.module) else {
                continue;
            };
            let Some(record) = module.records.get(stack.item.record) else {
                continue;
            };

            let wrapper = ContextPanel::new()
                .spawn_menu(spawn_context_menu(StackState {
                    id,
                    is_equipped: stack.equipped,
                    events: self.events.clone(),
                }))
                .mount(&root);

            let label = format!("{} ({})", record.name, stack.quantity);
            Text::new(label).mount(&wrapper);
        }

        root
    }
}

#[derive(Clone, Debug)]
struct StackState {
    id: InventorySlotId,
    events: mpsc::Sender<UiEvent>,
    is_equipped: bool,
}

fn spawn_context_menu(stack: StackState) -> Callback<ContextMenuState> {
    Callback::from(move |state: ContextMenuState| {
        let root = state.ctx;

        if stack.is_equipped {
            let events = stack.events.clone();
            let button = Button::new()
                .on_click(move |_| {
                    events.send(unequip_event(stack.id)).unwrap();
                })
                .mount(&root);
            Text::new("Unequip").mount(&button);
        } else {
            let events = stack.events.clone();
            let button = Button::new()
                .on_click(move |_| {
                    events.send(equip_event(stack.id)).unwrap();
                })
                .mount(&root);
            Text::new("Equip").mount(&button);
        }

        let events = stack.events.clone();
        let button = Button::new()
            .on_click(move |_| {
                events.send(drop_event(stack.id)).unwrap();
            })
            .mount(&root);
        Text::new("Drop").mount(&button);
    })
}

#[derive(Debug)]
pub struct InventoryProxy {
    pub id: NodeId,
}

impl InventoryProxy {
    pub fn new(
        inventory: &Inventory,
        modules: Modules,
        ui_ctx: &mut UiRootContext,
        tx: mpsc::Sender<UiEvent>,
    ) -> Self {
        let root = ui_ctx.append(InventoryUi {
            inventory,
            modules,
            events: tx,
        });

        Self {
            id: root.node().unwrap(),
        }
    }
}

fn equip_event(slot: InventorySlotId) -> UiEvent {
    let mut data = Vec::new();
    data.extend(slot.into_raw().to_le_bytes());

    UiEvent {
        id: EVENT_EQUIP,
        data,
    }
}

fn unequip_event(slot: InventorySlotId) -> UiEvent {
    let mut data = Vec::new();
    data.extend(slot.into_raw().to_le_bytes());

    UiEvent {
        id: EVENT_UNEQUIP,
        data,
    }
}

fn drop_event(slot: InventorySlotId) -> UiEvent {
    let mut data = Vec::new();
    data.extend(slot.into_raw().to_le_bytes());

    UiEvent {
        id: EVENT_DROP,
        data,
    }
}
