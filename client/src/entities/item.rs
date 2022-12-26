//! A world item
//!
//!
//!

use std::sync::Arc;

use bevy::prelude::{AssetServer, Bundle, DespawnRecursiveExt, Entity, World};
use bevy::scene::SceneBundle;
use common::components::interaction::{Interaction, InteractionId, Interactions};
use common::components::inventory::Inventory;
use common::components::items::Item;

use crate::bundles::PhysicsBundle;

#[derive(Bundle)]
pub struct ItemBundle {
    #[bundle]
    pub transform: crate::bundles::TransformBundle,
    #[bundle]
    pub scene: SceneBundle,
    #[bundle]
    pub physics: PhysicsBundle,

    pub item: Item,
    /// Item can be picked up.
    pub interactions: Interactions,
}

impl ItemBundle {
    pub fn new(assets: &AssetServer, item: Item) -> Self {
        let interaction = Interaction {
            id: InteractionId(0.into()),
            text: Some("Take".to_owned()),
            executor: Arc::new(pickup),
        };

        Self {
            transform: crate::bundles::TransformBundle::new(),
            scene: SceneBundle {
                scene: assets.load("barrel.glb#Scene0"),
                ..Default::default()
            },
            physics: PhysicsBundle::new(),
            item,
            interactions: interaction.into(),
        }
    }
}

fn pickup(target: Entity, actor: Entity, world: &mut World) {
    // FIXME: Optimally this would only get removed after the actor inventory was
    // acquired.
    let item = world.entity_mut(target).remove::<Item>().unwrap();

    let Ok(mut inventory) = world.query::<&mut Inventory>().get_mut(world, actor) else {
        // If the actor doesn't exist anymore, we discard the interaction.
        return;
    };

    // TODO: Error handling
    let _ = inventory.insert(item);

    world.entity_mut(target).despawn_recursive();
}
