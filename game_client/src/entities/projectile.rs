use bevy::prelude::{
    AssetServer, Bundle, Commands, ComputedVisibility, GlobalTransform, Handle, Res, Transform,
    Visibility,
};
use bevy::scene::{Scene, SceneBundle};
use game_common::components::projectile::Projectile;

#[derive(Bundle)]
pub struct ProjectileBundle {
    #[bundle]
    pub scene: SceneBundle,
    pub projectile: Projectile,
}

impl ProjectileBundle {
    pub fn new(assets: Res<AssetServer>) -> Self {
        let scene = assets.load("bullet.glb#Scene0");

        Self {
            projectile: Projectile,
            scene: SceneBundle {
                scene,
                ..Default::default()
            },
        }
    }
}

pub struct ProjectileBuilder {
    // POS
    pub transform: Transform,
    pub global_transform: GlobalTransform,

    // GFX
    pub scene: Option<Handle<Scene>>,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,

    // MARKER
    pub projectile: Projectile,
}

impl ProjectileBuilder {
    // TODO: This should become const once possible.
    pub fn new() -> Self {
        Self {
            transform: Transform::IDENTITY,
            global_transform: GlobalTransform::IDENTITY,
            scene: None,
            visibility: Visibility::Inherited,
            computed_visibility: ComputedVisibility::HIDDEN,
            projectile: Projectile,
        }
    }

    pub const fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn spawn(self, commands: &mut Commands) {
        let Self {
            transform,
            global_transform,
            scene,
            visibility,
            computed_visibility,
            projectile,
        } = self;

        let mut builder = commands.spawn_empty();
        builder.insert(transform);
        builder.insert(global_transform);

        if let Some(scene) = scene {
            builder.insert(scene);
            builder.insert(visibility);
            builder.insert(computed_visibility);
        }

        builder.insert(projectile);
    }
}
