use bevy::prelude::{AssetServer, Bundle, Transform, Vec3};
use bevy::scene::SceneBundle;
use bevy_rapier3d::prelude::{Collider, RigidBody};

#[derive(Bundle)]
pub struct ObjectBundle {
    pub rigid_body: RigidBody,
    pub collider: Collider,
    #[bundle]
    pub scene: SceneBundle,
}

impl ObjectBundle {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            rigid_body: RigidBody::Fixed,
            collider: Collider::cuboid(0.05, 0.5, 0.5),
            scene: SceneBundle {
                scene: assets.load("wall.glb#Scene0"),
                transform: Transform {
                    translation: Vec3::new(0.0, 0.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[inline]
    pub fn at(mut self, translation: Vec3) -> Self {
        self.scene.transform.translation = translation;
        self
    }
}
