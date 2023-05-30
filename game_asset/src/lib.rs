//! Asset loader
//!

mod asset;
mod io;

use bevy_app::{App, Plugin};
use bevy_ecs::system::Resource;

pub use crate::asset::{Asset, Assets, Handle, HandleId};

#[derive(Clone, Debug, Default)]
pub struct AssetPlugin {}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetServer::new());
    }
}

pub trait AssetAppExt {
    fn add_asset<T: Asset>(&mut self);
}

#[derive(Debug, Resource)]
pub struct AssetServer {}

impl AssetServer {
    pub fn new() -> Self {
        Self {}
    }
}
