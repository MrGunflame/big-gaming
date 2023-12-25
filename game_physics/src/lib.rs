pub mod data;
pub mod query;

mod control;
mod convert;
mod handle;
mod pipeline;

use std::collections::HashMap;
use std::fmt::Debug;

use control::CharacterController;
use convert::{point, quat, rotation, vec3, vector};
use game_common::components::physics::{ColliderShape, RigidBody, RigidBodyKind};
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::events::{self, Event, EventQueue};
use game_common::world::World;
use game_tracing::trace_span;
use glam::{Quat, Vec3};
use handle::HandleMap;
use nalgebra::{Isometry, OPoint};
use parking_lot::Mutex;
use rapier3d::parry::shape::Cuboid;
use rapier3d::prelude::{
    BroadPhase, CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet, CollisionEvent,
    ContactPair, EventHandler, ImpulseJointSet, IntegrationParameters, IslandManager,
    MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryFilter, QueryPipeline, Ray,
    RigidBodyBuilder, RigidBodyHandle, RigidBodySet, RigidBodyType, Vector,
};

pub struct Pipeline {
    // Physics engine shit
    pipeline: PhysicsPipeline,
    gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    // Our shit
    /// Set of rigid bodies attached to entities.
    body_handles: HandleMap<RigidBodyHandle>,
    /// Set of colliders attached to entities.
    // We need the collider for collision events.
    collider_handles: HandleMap<ColliderHandle>,
    event_handler: CollisionHandler,
    controllers: HashMap<RigidBodyHandle, CharacterController>,
}

impl Pipeline {
    pub fn new() -> Self {
        let integration_parameters = IntegrationParameters {
            dt: 1.0 / 60.0,
            min_ccd_dt: 1.0 / 60.0 / 100.0,
            ..Default::default()
        };

        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector::new(0.0, -9.81, 0.0),
            integration_parameters,
            islands: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            body_handles: HandleMap::new(),
            event_handler: CollisionHandler::new(),
            collider_handles: HandleMap::new(),
            controllers: HashMap::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn step(&mut self, world: &mut World, events: &mut EventQueue) {
        let _span = trace_span!("Pipeline::step").entered();

        self.update_rigid_bodies(world);
        self.update_colliders(world);

        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            None,
            &(),
            &self.event_handler,
        );

        self.drive_controllers();

        self.emit_events(events);
        self.write_back(world);
    }

    fn update_rigid_bodies(&mut self, world: &World) {
        let _span = trace_span!("PhysicsPipeline::update_rigid_bodies").entered();

        let mut despawned_entities = self.body_handles.clone();

        for (entity, (transform, rigid_body)) in world.query::<(Transform, RigidBody)>() {
            let Some(handle) = self.body_handles.get(entity) else {
                let kind = match rigid_body.kind {
                    RigidBodyKind::Fixed => RigidBodyType::Fixed,
                    RigidBodyKind::Dynamic => RigidBodyType::Dynamic,
                    RigidBodyKind::Kinematic => RigidBodyType::KinematicVelocityBased,
                };

                let mut builder = RigidBodyBuilder::new(kind);
                builder = builder.position(Isometry {
                    translation: vector(transform.translation).into(),
                    rotation: rotation(transform.rotation),
                });

                let body_handle = self.bodies.insert(builder);
                self.body_handles.insert(entity, body_handle);
                continue;
            };

            let body = self.bodies.get_mut(handle).unwrap();

            let translation = vector(transform.translation);
            if *body.translation() != translation {
                body.set_translation(translation, true);
            }

            let rotation = rotation(transform.rotation);
            if *body.rotation() != rotation {
                body.set_rotation(rotation, true);
            }

            // FIXME: Handle updated rigid body parameters.

            despawned_entities.remove(entity);
        }

        for (entity, handle) in despawned_entities.iter() {
            self.body_handles.remove(entity);
            self.bodies.remove(
                handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                // Don't remove attached colliders, they are removed in the next
                // stage that updates all entities with colliders.
                false,
            );
        }
    }

    fn update_colliders(&mut self, world: &World) {
        let _span = trace_span!("PhysicsPipeline::update_colliders").entered();

        let mut despawned_entities = self.collider_handles.clone();

        for (entity, (transform, collider)) in
            world.query::<(Transform, game_common::components::physics::Collider)>()
        {
            let Some(handle) = self.collider_handles.get(entity) else {
                let mut builder = match collider.shape {
                    ColliderShape::Cuboid(cuboid) => {
                        ColliderBuilder::cuboid(cuboid.hx, cuboid.hy, cuboid.hz)
                    }
                };

                builder = builder.position(Isometry {
                    translation: vector(transform.translation).into(),
                    rotation: rotation(transform.rotation),
                });

                let handle = self.colliders.insert(builder);
                self.collider_handles.insert(entity, handle);

                let body = self.body_handles.get(entity).unwrap();
                self.colliders
                    .set_parent(handle, Some(body), &mut self.bodies);

                continue;
            };

            let state = self.colliders.get_mut(handle).unwrap();

            let translation = vector(transform.translation);
            if *state.translation() != translation {
                state.set_translation(translation);
            }

            let rotation = rotation(transform.rotation);
            if *state.rotation() != rotation {
                state.set_rotation(rotation);
            }

            // TODO: Handle updated collider parameters.

            let body = self.body_handles.get(entity).unwrap();
            self.colliders
                .set_parent(handle, Some(body), &mut self.bodies);

            despawned_entities.remove(entity);
        }

        for (entity, handle) in despawned_entities.iter() {
            self.collider_handles.remove(entity);
            self.colliders
                .remove(handle, &mut self.islands, &mut self.bodies, true);
        }
    }

    fn write_back(&mut self, world: &mut World) {
        for (handle, body) in self.bodies.iter() {
            let entity = self.body_handles.get2(handle).unwrap();

            let mut transform = world.get_typed::<Transform>(entity);
            transform.translation = vec3(*body.translation());
            transform.rotation = quat(*body.rotation());
            world.insert_typed(entity, transform);
        }
    }

    fn emit_events(&mut self, queue: &mut EventQueue) {
        let events = self.event_handler.events.get_mut();

        for event in &*events {
            let lhs = self.collider_handles.get2(event.handles[0]).unwrap();
            let rhs = self.collider_handles.get2(event.handles[1]).unwrap();

            queue.push(Event::Collision(events::CollisionEvent {
                entity: lhs,
                other: rhs,
            }));
        }

        events.clear();
    }

    fn drive_controllers(&mut self) {
        self.query_pipeline.update(&self.bodies, &self.colliders);

        for (handle, controller) in &self.controllers {
            let collider = self.bodies.get(*handle).unwrap().colliders()[0];

            controller.apply_gravity(
                self.integration_parameters.dt,
                &mut self.bodies,
                &self.colliders,
                *handle,
                collider,
                &self.query_pipeline,
            );

            // Gravity may move an entity, so we need to rebuild the pipeline.
            self.query_pipeline.update(&self.bodies, &self.colliders);
        }
    }

    pub fn cast_ray(
        &self,
        ray: game_common::math::Ray,
        max_toi: f32,
        filter: query::QueryFilter,
    ) -> Option<(EntityId, f32)> {
        let ray = Ray {
            origin: point(ray.origin),
            dir: vector(ray.direction),
        };

        let pred = |handle, collider: &Collider| {
            let entity = self.collider_handles.get2(handle).unwrap();
            if filter.exclude_entities.contains(&entity) {
                false
            } else {
                true
            }
        };
        let filter = QueryFilter::new().predicate(&pred);

        match self.query_pipeline.cast_ray(
            &self.bodies,
            &self.colliders,
            &ray,
            max_toi,
            true,
            filter,
        ) {
            Some((handle, toi)) => {
                let entity = self.collider_handles.get2(handle).unwrap();
                Some((entity, toi))
            }
            None => None,
        }
    }

    pub fn cast_shape(
        &self,
        translation: Vec3,
        rot: Quat,
        direction: Vec3,
        max_toi: f32,
        shape: ColliderShape,
        filter: query::QueryFilter,
    ) -> Option<(EntityId, f32)> {
        let shape_origin = Isometry {
            rotation: rotation(rot),
            translation: vector(translation).into(),
        };
        let shape_vel = vector(direction);

        let pred = |handle, collider: &Collider| {
            let entity = self.collider_handles.get2(handle).unwrap();
            if filter.exclude_entities.contains(&entity) {
                false
            } else {
                true
            }
        };
        let filter = QueryFilter::new().predicate(&pred);

        let shape = match shape {
            ColliderShape::Cuboid(cuboid) => {
                let half_extents = vector(Vec3::new(cuboid.hx, cuboid.hy, cuboid.hz));
                Cuboid::new(half_extents)
            }
        };

        match self.query_pipeline.cast_shape(
            &self.bodies,
            &self.colliders,
            &shape_origin,
            &shape_vel,
            &shape,
            max_toi,
            true,
            filter,
        ) {
            Some((handle, toi)) => {
                let entity = self.collider_handles.get2(handle).unwrap();
                Some((entity, toi.toi))
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct CollisionHandler {
    events: Mutex<Vec<Collision>>,
}

impl CollisionHandler {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::with_capacity(16)),
        }
    }
}

impl EventHandler for CollisionHandler {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        match event {
            CollisionEvent::Started(lhs, rhs, _) => {
                let collision = Collision {
                    handles: [lhs, rhs],
                };

                self.events.lock().push(collision);
            }
            CollisionEvent::Stopped(_lhs, _rhs, _) => (),
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: rapier3d::prelude::Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &rapier3d::prelude::ContactPair,
        _total_force_magnitude: rapier3d::prelude::Real,
    ) {
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Collision {
    handles: [ColliderHandle; 2],
}

fn extract_actor_rotation(input: Quat) -> Quat {
    let mut direction = input * -Vec3::Z;
    direction.y = direction.y.clamp(-1.0, 1.0);
    let angle = if direction.x.is_sign_negative() {
        -direction.y.asin()
    } else {
        direction.y.asin()
    };

    Quat::from_axis_angle(Vec3::Y, angle)
}

impl Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use game_common::components::physics::{
        Collider, ColliderShape, Cuboid, RigidBody, RigidBodyKind,
    };
    use game_common::components::transform::Transform;
    use game_common::events::EventQueue;
    use game_common::world::World;

    use crate::Pipeline;

    #[test]
    fn dynamic_rigid_body_with_collider() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert_typed(
            entity,
            RigidBody {
                kind: RigidBodyKind::Dynamic,
            },
        );
        world.insert_typed(
            entity,
            Collider {
                friction: 1.0,
                restitution: 1.0,
                shape: ColliderShape::Cuboid(Cuboid {
                    hx: 1.0,
                    hy: 1.0,
                    hz: 1.0,
                }),
            },
        );
        world.insert_typed(entity, Transform::IDENTITY);

        let mut events = EventQueue::new();
        let mut pipeline = Pipeline::new();
        pipeline.step(&mut world, &mut events);

        let transform = world.get_typed::<Transform>(entity);
        assert_ne!(transform, Transform::IDENTITY);
    }
}
