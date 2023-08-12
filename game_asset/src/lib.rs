//! Asset loader
//!

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod asset;
mod server;

use bevy_app::{App, Plugin};

pub use crate::asset::{Asset, Assets, Handle, HandleId};
pub use server::AssetServer;

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
