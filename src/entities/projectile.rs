use bevy::prelude::{
    AssetServer, Bundle, Commands, Component, ComputedVisibility, GlobalTransform, Handle,
    PbrBundle, Res, Transform, Vec3, Visibility,
};
use bevy::scene::{Scene, SceneBundle};
use bevy_rapier3d::prelude::{AdditionalMassProperties, Ccd, Collider, RigidBody, Velocity};

/// A marker component for damage-carrying projectile.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct Projectile;

/// A list of incoming damage to an actor.
pub struct IncomingDamage {
    incoming: Vec<Damage>,
}

impl IncomingDamage {
    pub fn push(&mut self, damage: Damage) {
        self.incoming.push(damage);
    }

    pub fn pop(&mut self) -> Option<Damage> {
        self.incoming.pop()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Damage {
    // placeholder
    pub class: Option<()>,
    /// The amount of damage applied.
    pub amount: u32,
}

impl Damage {
    #[inline]
    pub fn new(amount: u32) -> Self {
        Self {
            class: None,
            amount,
        }
    }
}

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
        let scene = assets.load("bricks.glb#Scene0");
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

pub struct ProjectileBuilder {
    // POS
    pub transform: Transform,
    pub global_transform: GlobalTransform,

    // GFX
    pub scene: Option<Handle<Scene>>,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,

    // PHYSICS
    pub collider: Collider,
    pub velocity: Velocity,
    pub ccd: Ccd,
    pub mass: AdditionalMassProperties,
    pub rigid_body: RigidBody,

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
            visibility: Visibility::INVISIBLE,
            computed_visibility: ComputedVisibility::INVISIBLE,
            collider: Collider::cuboid(1.0, 1.0, 1.0),
            velocity: Velocity::zero(),
            ccd: Ccd::enabled(),
            mass: AdditionalMassProperties::Mass(0.0),
            rigid_body: RigidBody::Dynamic,
            projectile: Projectile,
        }
    }

    pub const fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    // TODO: Project velocity along transform.rotation.
    pub fn velocity(mut self, velocity: Velocity) -> Self {
        self.velocity = velocity;
        self
    }

    // TODO: Allow passing MassProperties.
    pub fn mass(mut self, mass: f32) -> Self {
        self.mass = AdditionalMassProperties::Mass(mass);
        self
    }

    pub fn spawn(self, commands: &mut Commands) {
        let Self {
            transform,
            global_transform,
            scene,
            visibility,
            computed_visibility,
            collider,
            velocity,
            ccd,
            mass,
            rigid_body,
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

        builder.insert(collider);
        builder.insert(velocity);
        builder.insert(ccd);
        builder.insert(mass);
        builder.insert(rigid_body);
        builder.insert(projectile);
    }
}
