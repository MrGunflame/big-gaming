use std::time::Duration;

use bevy::prelude::{Bundle, Vec3};
use game_common::components::actor::{
    Actor, ActorFigure, ActorFlags, MovementSpeed, SpawnPoint, SpawnPoints,
};
use game_common::components::animation::AnimationQueue;
use game_common::components::inventory::{Equipment, EquipmentSlot, Inventory};
use game_common::components::items::{Cooldown, Item, ItemId, Magazine};

use crate::bundles::VisibilityBundle;
use crate::plugins::combat::CombatBundle;

#[derive(Bundle)]
pub struct ActorBundle {
    #[bundle]
    pub transform: crate::bundles::TransformBundle,
    #[bundle]
    pub visibility: VisibilityBundle,
    // #[bundle]
    // pub scene: SceneBundle,
    #[bundle]
    pub combat: CombatBundle,

    pub actor: Actor,
    pub actor_state: ActorFlags,
    pub movement_speed: MovementSpeed,
    pub actor_figure: ActorFigure,
    pub inventory: Inventory,
    pub equipment: Equipment,
    pub animation_queue: AnimationQueue,
    pub spawn_points: SpawnPoints,
}

impl ActorBundle {
    pub fn new() -> Self {
        let mut equipment = Equipment::new();
        equipment.insert(
            EquipmentSlot::MAIN_HAND,
            Item {
                id: ItemId(0.into()),
                resistances: None,
                damage: None,
                components: None,
                magazine: Some(Magazine::Single {
                    id: ItemId(64.into()),
                    count: 31,
                }),
                mass: Default::default(),
                ammo: Some(ItemId(64.into())),
                cooldown: Cooldown::new(Duration::new(0, 200)),
                actions: Default::default(),
            },
        );

        Self {
            transform: crate::bundles::TransformBundle::from_translation(Vec3::new(
                10.0, 1.0, 10.0,
            )),
            // scene: SceneBundle {
            //     scene: assets.load("person.glb#Scene0"),
            //     ..Default::default()
            // },
            visibility: VisibilityBundle::new(),
            actor: Actor,
            combat: CombatBundle::new(),
            actor_state: ActorFlags::default(),
            movement_speed: MovementSpeed(3.0),
            actor_figure: ActorFigure {
                eyes: Vec3::new(0.0, 1.65, -0.3),
            },
            inventory: Inventory::new(),
            equipment,
            animation_queue: AnimationQueue::new(),
            spawn_points: SpawnPoints::from(SpawnPoint {
                translation: Vec3::splat(0.0),
                weight: 0,
            }),
        }
    }
}
