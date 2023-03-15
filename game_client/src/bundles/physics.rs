use bevy::prelude::Bundle;
use bevy_rapier3d::prelude::{Ccd, Collider, RigidBody, Velocity};

#[derive(Bundle)]
pub struct PhysicsBundle {
    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub ccd: Ccd,
    pub collider: Collider,
    // pub mass: AdditionalMassProperties,
}

impl PhysicsBundle {
    pub fn new() -> Self {
        Self {
            rigid_body: RigidBody::Dynamic,
            velocity: Velocity::zero(),
            ccd: Ccd::enabled(),
            collider: Collider::cuboid(1.0, 1.0, 1.0),
        }
    }
}
