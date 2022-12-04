use bevy::prelude::{AssetServer, Bundle, Component, PbrBundle, Res, Vec3};
use bevy::scene::{Scene, SceneBundle};
use bevy_rapier3d::prelude::{AdditionalMassProperties, Ccd, Collider, RigidBody, Velocity};

/// A marker component for damage-carrying projectile.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct Projectile;

#[derive(Bundle)]
pub struct ProjectileBundle {
    #[bundle]
    pub scene: SceneBundle,
    pub velocity: Velocity,
    pub ccd: Ccd,
    pub collider: Collider,
    pub mass: AdditionalMassProperties,
    pub rigid_body: RigidBody,
    pub projectile: Projectile,
}

impl ProjectileBundle {
    pub fn new(assets: Res<AssetServer>) -> Self {
        let scene = assets.load("thing.glb#Scene0");
        let velocity = Velocity::zero();
        let mass = AdditionalMassProperties::Mass(3.56);
        let ccd = Ccd::enabled();
        let collider = Collider::cuboid(1.0, 1.0, 1.0);

        Self {
            scene: SceneBundle {
                scene,
                ..Default::default()
            },
            ccd,
            collider,
            projectile: Projectile,
            rigid_body: RigidBody::Dynamic,
            mass,
            velocity,
        }
    }
}
