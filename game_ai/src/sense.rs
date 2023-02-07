use bevy::prelude::{App, Query, Transform, With};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use game_common::components::actor::Actor;
use game_common::components::combat::Resistances;
use game_common::components::inventory::{Equipment, EquipmentSlot};
use game_common::components::items::Item;

pub(super) fn senses(app: &mut App) {
    app.add_system(vision_sense).add_system(equipment_sense);
}

#[derive(Clone, Debug, Default, Component)]
pub struct Vision {
    pub entities: Vec<Transform>,
}

impl Vision {
    pub const fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

/// Equipment of the AI host.
#[derive(Clone, Debug, Default, Component)]
pub struct HostEquipment {
    pub resistances: Resistances,
    pub weapons: Vec<Item>,
}

fn vision_sense(
    mut hosts: Query<(Entity, &Transform, &mut Vision)>,
    mut actors: Query<(Entity, &Transform), With<Actor>>,
) {
    for (entity, host, mut vision) in &mut hosts {
        for (actor, actor_transform) in &mut actors {
            // Skip the host itself.
            if entity == actor {
                continue;
            }

            vision.entities.clear();

            let distance = actor_transform.translation - host.translation;

            if distance.length() < 10.0 {
                vision.entities.push(*actor_transform);
            }
        }
    }
}

fn equipment_sense(mut hosts: Query<(&Equipment, &mut HostEquipment)>) {
    for (equipment, mut host_equipment) in &mut hosts {
        let mut resistances = Resistances::new();
        let weapons = equipment
            .get(EquipmentSlot::MAIN_HAND)
            .map(|item| vec![item.clone()])
            .unwrap_or_default();

        for item in equipment {
            // Collect all resistances from all equipped items.
            if let Some(res) = &item.resistances {
                resistances += res;
            }
        }

        *host_equipment = HostEquipment {
            resistances,
            weapons,
        }
    }
}
