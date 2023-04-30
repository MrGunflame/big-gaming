use game_common::components::actions::{ActionId, Actions};
use game_common::components::components::{Component, Components};
use game_common::components::items::{Item, ItemId};
use game_common::world::inventory::InventoryMut;
use game_core::modules::Modules;

pub fn add_item(inventory: &mut InventoryMut<'_>, id: ItemId, modules: &Modules) {
    let module = modules.get(id.0.module).unwrap();
    let base_item = module
        .records
        .get(id.0.record)
        .unwrap()
        .body
        .clone()
        .unwrap_item();

    let mut actions = Actions::new();
    for id in base_item.actions {
        actions.push(ActionId(id));
    }

    let mut components = Components::new();
    for comp in base_item.components {
        components.insert(comp.record, Component { bytes: comp.value });
    }

    let item = Item {
        id,
        mass: base_item.mass,
        resistances: Default::default(),
        components,
        actions,
        equipped: false,
        hidden: false,
    };

    inventory.insert(item).unwrap();
}
