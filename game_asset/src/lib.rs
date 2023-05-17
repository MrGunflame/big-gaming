//! Asset loader

use std::collections::HashMap;
use std::sync::Arc;

use bevy_ecs::system::Resource;

#[derive(Debug, Resource)]
pub struct AssetServer {
    assets: HashMap<HandleId, Asset>,
}

impl AssetServer {
    pub fn get(&self, handle: &Handle) -> Option<&Asset> {
        self.assets.get(&handle.id)
    }

    pub fn contains(&self, handle: &Handle) -> bool {
        self.assets.contains_key(&handle.id)
    }
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub bytes: Arc<[u8]>,
}

#[derive(Clone, Debug)]
pub struct Handle {
    id: HandleId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HandleId(u64);
