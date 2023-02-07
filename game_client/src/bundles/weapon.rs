use bevy::prelude::{AssetServer, Bundle};
use bevy::scene::SceneBundle;

#[derive(Bundle)]
pub struct WeaponBundle {
    #[bundle]
    pub scene: SceneBundle,
}

impl WeaponBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            scene: SceneBundle {
                scene: assets.load("ar.glb#Scene0"),
                ..Default::default()
            },
        }
    }
}
