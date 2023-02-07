use bevy_ecs::bundle::Bundle;
use bevy_rapier3d::prelude::{Ccd, Collider, RigidBody, Velocity};

/// Physics bundle for dynamic bodies.
#[derive(Clone, Debug, Bundle)]
pub struct DynamicPhysicsBundle {
    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub collider: Collider,
    pub ccd: Ccd,
}

impl Default for DynamicPhysicsBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::Dynamic,
            velocity: Velocity::zero(),
            collider: Collider::default(),
            ccd: Ccd::enabled(),
        }
    }
}

/// Physics bundle for static (immovable) bodies.
#[derive(Clone, Debug, Bundle)]
pub struct StaticPhysicsBundle {
    pub rigid_body: RigidBody,
    pub collider: Collider,
}

impl Default for StaticPhysicsBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::Fixed,
            collider: Collider::default(),
        }
    }
}
