pub mod data;
pub mod query;

mod convert;
mod handle;
mod pipeline;

use std::collections::HashMap;
use std::fmt::Debug;

use convert::{point, quat, rotation, vec3, vector};
use game_common::components::{Axis, Children, ColliderShape, RigidBody, RigidBodyKind, Transform};
use game_common::entity::EntityId;
use game_common::events::{self, Event, EventQueue};
use game_common::world::{QueryWrapper, World};
use game_tracing::trace_span;
use glam::{Quat, Vec3};
use handle::HandleMap;
use nalgebra::Isometry;
use parking_lot::Mutex;
use rapier3d::parry::shape::{Ball, Capsule, Cuboid};
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
    /// Child => Parent
    body_parents: HashMap<EntityId, EntityId>,
    body_children: HashMap<EntityId, Vec<EntityId>>,
    /// Set of colliders attached to entities.
    // We need the collider for collision events.
    collider_handles: HandleMap<ColliderHandle>,
    event_handler: CollisionHandler,
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
            query_pipeline: QueryPipeline::new(),
            body_parents: HashMap::new(),
            body_children: HashMap::new(),
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

        self.query_pipeline.update(&self.bodies, &self.colliders);

        self.emit_events(events);
        self.write_back(world);
    }

    fn update_rigid_bodies(&mut self, world: &World) {
        let _span = trace_span!("PhysicsPipeline::update_rigid_bodies").entered();

        let mut despawned_entities = self.body_handles.clone();

        for (entity, QueryWrapper((transform, rigid_body))) in
            world.query::<QueryWrapper<(Transform, RigidBody)>>()
        {
            let children: Children = world.get_typed(entity).unwrap_or_default();

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

            // Remove previous children before updating.
            if let Some(children) = self.body_children.remove(&entity) {
                for children in children {
                    self.body_parents.remove(&children);
                }
            }

            self.body_children.insert(
                entity,
                children
                    .get()
                    .iter()
                    .map(|v| EntityId::from_raw(v.into_raw()))
                    .collect(),
            );
            for children in children.get() {
                self.body_parents
                    .insert(EntityId::from_raw(children.into_raw()), entity);
            }

            despawned_entities.remove(entity);
        }

        for (entity, handle) in despawned_entities.iter() {
            if let Some(children) = self.body_children.remove(&entity) {
                for children in children {
                    self.body_parents.remove(&children);
                }
            }

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

        for (entity, QueryWrapper((transform, collider))) in
            world.query::<QueryWrapper<(Transform, game_common::components::Collider)>>()
        {
            let Some(handle) = self.collider_handles.get(entity) else {
                let Some(body) = self.get_collider_parent(entity) else {
                    tracing::warn!("collider for entity {:?} is missing rigid body", entity);
                    continue;
                };

                let mut builder = match collider.shape {
                    ColliderShape::Cuboid(cuboid) => {
                        ColliderBuilder::cuboid(cuboid.hx, cuboid.hy, cuboid.hz)
                    }
                    ColliderShape::Ball(ball) => ColliderBuilder::ball(ball.radius),
                    ColliderShape::Capsule(capsule) => match capsule.axis {
                        Axis::X => ColliderBuilder::capsule_x(capsule.half_height, capsule.radius),
                        Axis::Y => ColliderBuilder::capsule_y(capsule.half_height, capsule.radius),
                        Axis::Z => ColliderBuilder::capsule_z(capsule.half_height, capsule.radius),
                    },
                };

                builder = builder.position(Isometry {
                    translation: vector(transform.translation).into(),
                    rotation: rotation(transform.rotation),
                });

                let handle = self.colliders.insert(builder);
                self.collider_handles.insert(entity, handle);

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

            if state.friction() != collider.friction {
                state.set_friction(collider.friction);
            }

            if state.restitution() != collider.restitution {
                state.set_restitution(collider.restitution);
            }

            // TODO: Handle updated collider shape.

            let Some(body) = self.get_collider_parent(entity) else {
                tracing::warn!("collider for entity {:?} is missing rigid body", entity);
                continue;
            };

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

            let mut transform = world.get_typed::<Transform>(entity).unwrap();
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

    fn get_collider_parent(&self, entity: EntityId) -> Option<RigidBodyHandle> {
        // Select a rigid body for the collider. If the entity has a rigid body
        // it is preferred, otherwise we select the rigid body from the parent
        // entity.
        match self.body_handles.get(entity) {
            Some(handle) => Some(handle),
            None => match self.body_parents.get(&entity) {
                Some(parent) => Some(self.body_handles.get(*parent).unwrap()),
                None => None,
            },
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

        match shape {
            ColliderShape::Cuboid(cuboid) => {
                let half_extents = vector(Vec3::new(cuboid.hx, cuboid.hy, cuboid.hz));
                let shape = Cuboid::new(half_extents);

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
            ColliderShape::Ball(ball) => {
                let shape = Ball::new(ball.radius);

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
            ColliderShape::Capsule(capsule) => {
                let shape = match capsule.axis {
                    Axis::X => Capsule::new_x(capsule.half_height, capsule.radius),
                    Axis::Y => Capsule::new_y(capsule.half_height, capsule.radius),
                    Axis::Z => Capsule::new_z(capsule.half_height, capsule.radius),
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
    use game_common::components::{
        Collider, ColliderShape, Cuboid, RigidBody, RigidBodyKind, Transform,
    };
    use game_common::events::EventQueue;
    use game_common::world::World;
    use glam::Vec3;

    use crate::Pipeline;

    #[test]
    fn dynamic_rigid_body_with_collider() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert_typed(
            entity,
            RigidBody {
                kind: RigidBodyKind::Dynamic,
                linvel: Vec3::splat(0.0),
                angvel: Vec3::splat(0.0),
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

        let transform = world.get_typed::<Transform>(entity).unwrap();
        assert_ne!(transform, Transform::IDENTITY);
    }
}
