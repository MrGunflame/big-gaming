//! A world item
//!
//!
//!

use bevy::prelude::{AssetServer, Bundle};
use bevy::scene::SceneBundle;
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
}

impl ItemBundle {
    pub fn new(assets: &AssetServer, item: Item) -> Self {
        Self {
            transform: crate::bundles::TransformBundle::new(),
            scene: SceneBundle {
                scene: assets.load("bricks.glb#Scene0"),
                ..Default::default()
            },
            physics: PhysicsBundle::new(),
            item,
        }
    }
}
