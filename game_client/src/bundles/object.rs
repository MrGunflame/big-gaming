use bevy::prelude::{AssetServer, Bundle, Transform, Vec3};
use bevy::scene::SceneBundle;
use game_common::components::object::ObjectChildren;

#[derive(Bundle)]
pub struct ObjectBundle {
    #[bundle]
    pub scene: SceneBundle,
    pub children: ObjectChildren,
}

impl ObjectBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            scene: SceneBundle {
                scene: assets.load("wall.glb#Scene0"),
                transform: Transform {
                    translation: Vec3::new(0.0, 0.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            children: ObjectChildren::default(),
        }
    }

    #[inline]
    pub fn at(mut self, translation: Vec3) -> Self {
        self.scene.transform.translation = translation;
        self
    }
}
