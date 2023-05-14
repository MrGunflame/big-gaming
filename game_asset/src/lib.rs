//! Asset loader

use bevy_ecs::system::Resource;

#[derive(Debug, Resource)]
pub struct AssetServer {
    assets: Vec<Asset>,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub bytes: Vec<u8>,
}
