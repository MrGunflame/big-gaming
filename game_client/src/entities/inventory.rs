use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::system::{Commands, Query, Res};
use game_common::components::actions::Actions;
use game_common::components::components;
use game_common::components::components::Components;
use game_common::components::inventory::{Inventory, InventoryId};
use game_common::components::items::{Item, ItemId};
use game_core::modules::Modules;

#[derive(Clone, Debug, Component)]
pub struct AddInventoryItem {
    pub entity: Entity,
    pub slot: InventoryId,
    pub id: ItemId,
}

#[derive(Clone, Debug, Component)]
pub struct RemoveInventoryItem {
    pub entity: Entity,
    pub slot: InventoryId,
}

#[derive(Clone, Debug, Component)]
pub struct DestroyInventory {
    pub entity: Entity,
}

pub fn add_inventory_item(
    mut commands: Commands,
    events: Query<(Entity, &AddInventoryItem)>,
    mut actors: Query<&mut Inventory>,
    modules: Res<Modules>,
) {
    for (entity, event) in events.iter() {
        let mut inventory = actors.get_mut(event.entity).unwrap();

        let module = modules.get(event.id.0.module).unwrap();
        let record = module.records.get(event.id.0.record).unwrap();
        let item = record.clone().body.unwrap_item();

        let mut components = Components::new();
        for comp in item.components {
            components.insert(comp.record, components::Component { bytes: comp.value });
        }

        let mut actions = Actions::new();

        if let Err(err) = inventory.insert(Item {
            id: event.id,
            resistances: None,
            mass: item.mass,
            actions,
            components,
            equipped: false,
            hidden: false,
        }) {
            tracing::error!("failed to insert item into inventory: {}", err);
        }

        commands.entity(entity).despawn();
    }
}

pub fn remove_inventory_item(
    mut commands: Commands,
    events: Query<(Entity, &RemoveInventoryItem)>,
    mut actors: Query<&mut Inventory>,
) {
    for (entity, event) in events.iter() {
        let mut inventory = actors.get_mut(event.entity).unwrap();
        inventory.remove(event.slot);

        commands.entity(entity).despawn();
    }
}

pub fn destroy_inventory(
    mut commands: Commands,
    events: Query<(Entity, &DestroyInventory)>,
    mut actors: Query<&mut Inventory>,
) {
    for (entity, event) in events.iter() {
        let mut inventory = actors.get_mut(event.entity).unwrap();
        inventory.clear();

        commands.entity(entity).despawn();
    }
}
