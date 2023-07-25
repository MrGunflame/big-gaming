use game_common::components::transform::Transform;
use glam::Vec3;
use rapier3d::prelude::{
    ColliderHandle, ColliderSet, QueryFilter, QueryPipeline, RigidBodyHandle, RigidBodySet,
};

use crate::convert::vector;

#[derive(Clone, Debug)]
pub struct CharacterController {}

impl CharacterController {
    pub fn move_shape(
        &self,
        dt: f32,
        transform: Transform,
        translation: Vec3,
    ) -> CharacterMovement {
        todo!()
    }

    pub fn apply_gravity(
        &self,
        dt: f32,
        bodies: &mut RigidBodySet,
        colliders: &ColliderSet,
        body_handle: RigidBodyHandle,
        collider_hanlde: ColliderHandle,
        query_pipeline: &QueryPipeline,
    ) {
        let body = bodies.get(body_handle).unwrap();
        let collider = colliders.get(collider_hanlde).unwrap();

        let g = -9.81;
        let gravity = vector(Vec3::new(0.0, -1.0, 0.0));
        let v0 = body.linvel().y;

        // Max fall distance in timestep `t` is `v0 * t + 0.5 * g * pow(t, 2)`.
        let max_toi = v0 * dt + 0.5 * g * dt * dt;

        let filter = QueryFilter::new().exclude_rigid_body(body_handle);

        let (distance, hit) = match query_pipeline.cast_shape(
            bodies,
            colliders,
            body.position(),
            &gravity,
            collider.shape(),
            // Note that `cast_shape` uses `gravity * max_toi` as the final max toi,
            // but `gravity` is already negative and `max_toi` must not be negative
            // for the shape cast to return useful results.
            // This also means that the returned `toi` and hit must be negated.
            max_toi.abs(),
            true,
            filter,
        ) {
            Some((_, toi)) => (-toi.toi, true),
            None => (max_toi, false),
        };

        dbg!(distance);

        let body = bodies.get_mut(body_handle).unwrap();
        let mut position = *body.position();
        position.translation.y += distance;
        body.set_position(position, false);

        let mut linvel = *body.linvel();
        // Reset velocity back to 0 when an object is hit.
        if hit {
            linvel.y = 0.0;
        } else {
            linvel.y += g * dt;
        }
        body.set_linvel(linvel, false);
    }
}

/// Relative character movement
#[derive(Clone, Debug)]
pub struct CharacterMovement {
    pub translation: Vec3,
}

#[cfg(test)]
mod tests {
    use game_common::assert_approx_eq;
    use nalgebra::{Isometry, Quaternion, Unit};
    use rapier3d::prelude::{
        ColliderBuilder, ColliderHandle, ColliderSet, QueryPipeline, RigidBodyBuilder,
        RigidBodyHandle, RigidBodySet, RigidBodyType, Translation,
    };

    use super::CharacterController;

    struct TestPipeline {
        dt: f32,
        bodies: RigidBodySet,
        colliders: ColliderSet,
        body_handle: RigidBodyHandle,
        collider_handle: ColliderHandle,
        query_pipeline: QueryPipeline,
    }

    impl TestPipeline {
        fn new() -> Self {
            let mut bodies = RigidBodySet::new();
            let mut colliders = ColliderSet::new();

            let body_handle =
                bodies.insert(RigidBodyBuilder::new(RigidBodyType::KinematicVelocityBased).build());

            let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0).build();
            let collider_handle = colliders.insert_with_parent(collider, body_handle, &mut bodies);

            let mut query_pipeline = QueryPipeline::new();
            query_pipeline.update(&bodies, &colliders);

            Self {
                dt: 1.0 / 60.0,
                bodies,
                collider_handle,
                body_handle,
                colliders,
                query_pipeline,
            }
        }

        fn set_linvel(&mut self, y: f32) {
            let body = self.bodies.get_mut(self.body_handle).unwrap();
            let mut linvel = *body.linvel();
            linvel.y = y;
            body.set_linvel(linvel, false);

            self.query_pipeline.update(&self.bodies, &self.colliders);
        }

        fn insert_fixed(&mut self, collider: ColliderBuilder, translation: Translation<f32>) {
            let body_handle = self.bodies.insert(
                RigidBodyBuilder::new(RigidBodyType::Fixed)
                    .position(Isometry {
                        translation,
                        rotation: Unit::new_normalize(Quaternion::identity()),
                    })
                    .build(),
            );

            let collider = collider.build();
            self.colliders
                .insert_with_parent(collider, body_handle, &mut self.bodies);

            self.query_pipeline.update(&self.bodies, &self.colliders);
        }
    }

    #[test]
    fn apply_gravity_free_fall() {
        let mut pipeline = TestPipeline::new();
        let controller = CharacterController {};

        controller.apply_gravity(
            pipeline.dt,
            &mut pipeline.bodies,
            &pipeline.colliders,
            pipeline.body_handle,
            pipeline.collider_handle,
            &pipeline.query_pipeline,
        );

        let body = pipeline.bodies.get(pipeline.body_handle).unwrap();
        assert_approx_eq!(body.position().translation.y, -0.0013625);
        assert_approx_eq!(body.linvel().y, -0.1635);
    }

    #[test]
    fn apply_gravity_multi_step() {
        let mut pipeline = TestPipeline::new();
        let controller = CharacterController {};

        for _ in 0..60 {
            controller.apply_gravity(
                pipeline.dt,
                &mut pipeline.bodies,
                &pipeline.colliders,
                pipeline.body_handle,
                pipeline.collider_handle,
                &pipeline.query_pipeline,
            );
        }

        let body = pipeline.bodies.get(pipeline.body_handle).unwrap();
        assert_approx_eq!(body.position().translation.y, -4.9049993);
        assert_approx_eq!(body.linvel().y, -9.809996);
    }

    #[test]
    fn apply_gravity_hit() {
        let mut pipeline = TestPipeline::new();
        let controller = CharacterController {};

        pipeline.insert_fixed(
            ColliderBuilder::cuboid(100.0, 0.1, 100.0),
            Translation::new(0.0, -3.0, 0.0),
        );

        pipeline.set_linvel(-1000.0);

        controller.apply_gravity(
            pipeline.dt,
            &mut pipeline.bodies,
            &pipeline.colliders,
            pipeline.body_handle,
            pipeline.collider_handle,
            &pipeline.query_pipeline,
        );

        let body = pipeline.bodies.get(pipeline.body_handle).unwrap();
        // Character half extends = 1.0 + ground collider = 0.1 => 1.1 distance to plane.
        assert_approx_eq!(body.position().translation.y, -1.9);
        assert_approx_eq!(body.linvel().y, 0.0);
    }
}
