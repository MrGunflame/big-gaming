pub mod data;
pub mod query;

mod convert;
mod pipeline;

use std::collections::HashMap;
use std::fmt::Debug;

use convert::{point, quat, rotation, vec3, vector};
use game_common::collections::bimap::BiMap;
use game_common::components::{Axis, Children, ColliderShape, RigidBody, RigidBodyKind, Transform};
use game_common::entity::EntityId;
use game_common::events::{self, Event, EventQueue};
use game_common::world::{QueryWrapper, World};
use game_tracing::trace_span;
use glam::{Quat, Vec3};
use nalgebra::{Const, Isometry, OPoint};
use parking_lot::Mutex;
use query::QueryHit;
use rapier3d::geometry::{BroadPhaseMultiSap, TriMesh};
use rapier3d::math::Real;
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::parry::shape::{Ball, Capsule, Cuboid};
use rapier3d::prelude::{
    CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet, CollisionEvent, ContactPair,
    EventHandler, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet,
    NarrowPhase, PhysicsPipeline, QueryFilter, QueryPipeline, Ray, RigidBodyBuilder,
    RigidBodyHandle, RigidBodySet, RigidBodyType, SharedShape, Vector,
};

const DT: Real = 1.0 / 60.0;
const MIN_CCD_DT: Real = DT / 100.0;
const GRAVITY: Vector<Real> = Vector::new(0.0, -9.81, 0.0);

pub struct Pipeline {
    // Physics engine shit
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhaseMultiSap,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    // Our shit
    /// Set of rigid bodies attached to entities.
    body_handles: BiMap<EntityId, RigidBodyHandle>,
    /// Child => Parent
    body_parents: HashMap<EntityId, EntityId>,
    body_children: HashMap<EntityId, Vec<EntityId>>,
    /// Set of colliders attached to entities.
    // We need the collider for collision events.
    collider_handles: BiMap<EntityId, ColliderHandle>,
    event_handler: CollisionHandler,
}

impl Pipeline {
    pub fn new() -> Self {
        let integration_parameters = IntegrationParameters {
            dt: DT,
            min_ccd_dt: MIN_CCD_DT,
            ..Default::default()
        };

        Self {
            pipeline: PhysicsPipeline::new(),
            integration_parameters,
            islands: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            body_handles: BiMap::new(),
            event_handler: CollisionHandler::new(),
            collider_handles: BiMap::new(),
            query_pipeline: QueryPipeline::new(),
            body_parents: HashMap::new(),
            body_children: HashMap::new(),
        }
    }

    /// Returns entities with updated transform.
    pub fn step(&mut self, world: &mut World, events: &mut EventQueue) -> Vec<EntityId> {
        let _span = trace_span!("Pipeline::step").entered();

        self.update_rigid_bodies(world);
        self.update_colliders(world);

        self.pipeline.step(
            &GRAVITY,
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

        self.query_pipeline.update(&self.colliders);

        self.emit_events(events);
        self.write_back(world)
    }

    fn update_rigid_bodies(&mut self, world: &World) {
        let _span = trace_span!("PhysicsPipeline::update_rigid_bodies").entered();

        let mut despawned_entities = self.body_handles.clone();

        for (entity, QueryWrapper((transform, rigid_body))) in
            world.query::<QueryWrapper<(Transform, RigidBody)>>()
        {
            let children: Children = world.get_typed(entity).unwrap_or_default();

            let Some(handle) = self.body_handles.get_left(&entity) else {
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

            let body = self.bodies.get_mut(*handle).unwrap();

            let translation = vector(transform.translation);
            if *body.translation() != translation {
                body.set_translation(translation, true);
            }

            let rotation = rotation(transform.rotation);
            if *body.rotation() != rotation {
                body.set_rotation(rotation, true);
            }

            let linvel = vector(rigid_body.linvel);
            if *body.linvel() != linvel {
                body.set_linvel(linvel, true);
            }

            let angvel = vector(rigid_body.angvel);
            if *body.angvel() != angvel {
                body.set_angvel(angvel, true);
            }

            match rigid_body.kind {
                RigidBodyKind::Fixed => {
                    if body.body_type() != RigidBodyType::Fixed {
                        body.set_body_type(RigidBodyType::Fixed, true);
                    }
                }
                RigidBodyKind::Dynamic => {
                    if body.body_type() != RigidBodyType::Dynamic {
                        body.set_body_type(RigidBodyType::Dynamic, true);
                    }
                }
                RigidBodyKind::Kinematic => {
                    if body.body_type() != RigidBodyType::KinematicVelocityBased {
                        body.set_body_type(RigidBodyType::KinematicVelocityBased, true);
                    }
                }
            }

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

            despawned_entities.remove_left(&entity);
        }

        for (entity, handle) in despawned_entities.iter() {
            if let Some(children) = self.body_children.remove(&entity) {
                for children in children {
                    self.body_parents.remove(&children);
                }
            }

            self.body_handles.remove_left(entity);
            self.bodies.remove(
                *handle,
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
            let Some(handle) = self.collider_handles.get_left(&entity).copied() else {
                let Some(body) = self.get_collider_parent(entity) else {
                    tracing::warn!("collider for entity {:?} is missing rigid body", entity);
                    continue;
                };

                let mut builder = ColliderBuilder::new(build_shape(&collider.shape));

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

            despawned_entities.remove_left(&entity);
        }

        for (entity, handle) in despawned_entities.iter() {
            self.collider_handles.remove_left(entity);
            self.colliders
                .remove(*handle, &mut self.islands, &mut self.bodies, true);
        }
    }

    fn write_back(&mut self, world: &mut World) -> Vec<EntityId> {
        let mut updated_entities = Vec::new();

        for (handle, body) in self.bodies.iter() {
            let entity = *self.body_handles.get_right(&handle).unwrap();

            let mut transform = world.get_typed::<Transform>(entity).unwrap();
            let translation = vec3(*body.translation());
            let rotation = quat(*body.rotation());

            if transform.translation != translation || transform.rotation != rotation {
                updated_entities.push(entity);
            }

            transform.translation = translation;
            transform.rotation = rotation;
            world.insert_typed(entity, transform);
        }

        updated_entities
    }

    fn emit_events(&mut self, queue: &mut EventQueue) {
        let events = self.event_handler.events.get_mut();

        for event in &*events {
            let lhs = *self.collider_handles.get_right(&event.handles[0]).unwrap();
            let rhs = *self.collider_handles.get_right(&event.handles[1]).unwrap();

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
        match self.body_handles.get_left(&entity) {
            Some(handle) => Some(*handle),
            None => self
                .body_parents
                .get(&entity)
                .map(|parent| *self.body_handles.get_left(parent).unwrap()),
        }
    }

    pub fn cast_ray(
        &self,
        ray: game_common::math::Ray,
        max_toi: f32,
        filter: &query::QueryFilter,
    ) -> Option<QueryHit> {
        let _span = trace_span!("PhysicsPipeline::cast_ray").entered();

        let ray = Ray {
            origin: point(ray.origin),
            dir: vector(ray.direction),
        };

        let pred = |handle, _collider: &Collider| {
            let entity = *self.collider_handles.get_right(&handle).unwrap();
            !filter.exclude_entities.contains(&entity)
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
                let entity = *self.collider_handles.get_right(&handle).unwrap();
                Some(QueryHit { entity, toi })
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
        shape: &ColliderShape,
        filter: &query::QueryFilter,
    ) -> Option<QueryHit> {
        let _span = trace_span!("PhysicsPipeline::cast_shape").entered();

        let shape_origin = Isometry {
            rotation: rotation(rot),
            translation: vector(translation).into(),
        };
        let shape_vel = vector(direction);

        let pred = |handle, _collider: &Collider| {
            let entity = self.collider_handles.get_right(&handle).unwrap();
            !filter.exclude_entities.contains(&entity)
        };
        let filter = QueryFilter::new().predicate(&pred);

        let options = ShapeCastOptions {
            max_time_of_impact: max_toi,
            target_distance: 0.0,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false,
        };

        let res = match shape {
            ColliderShape::Cuboid(cuboid) => {
                let half_extents = vector(Vec3::new(cuboid.hx, cuboid.hy, cuboid.hz));
                let shape = Cuboid::new(half_extents);

                self.query_pipeline.cast_shape(
                    &self.bodies,
                    &self.colliders,
                    &shape_origin,
                    &shape_vel,
                    &shape,
                    options,
                    filter,
                )
            }
            ColliderShape::Ball(ball) => {
                let shape = Ball::new(ball.radius);

                self.query_pipeline.cast_shape(
                    &self.bodies,
                    &self.colliders,
                    &shape_origin,
                    &shape_vel,
                    &shape,
                    options,
                    filter,
                )
            }
            ColliderShape::Capsule(capsule) => {
                let shape = match capsule.axis {
                    Axis::X => Capsule::new_x(capsule.half_height, capsule.radius),
                    Axis::Y => Capsule::new_y(capsule.half_height, capsule.radius),
                    Axis::Z => Capsule::new_z(capsule.half_height, capsule.radius),
                };

                self.query_pipeline.cast_shape(
                    &self.bodies,
                    &self.colliders,
                    &shape_origin,
                    &shape_vel,
                    &shape,
                    options,
                    filter,
                )
            }
            ColliderShape::TriMesh(mesh) => {
                let vertices = mesh
                    .vertices()
                    .iter()
                    .map(|vertex| OPoint::<f32, Const<3>>::new(vertex.x, vertex.y, vertex.z))
                    .collect();
                let indices = mesh
                    .indices()
                    .windows(3)
                    .map(|indices| indices.try_into().unwrap())
                    .collect();

                let shape = TriMesh::new(vertices, indices);

                self.query_pipeline.cast_shape(
                    &self.bodies,
                    &self.colliders,
                    &shape_origin,
                    &shape_vel,
                    &shape,
                    options,
                    filter,
                )
            }
        };

        res.map(|(handle, toi)| {
            let entity = *self.collider_handles.get_right(&handle).unwrap();
            QueryHit {
                entity,
                toi: toi.time_of_impact,
            }
        })
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
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

impl Default for CollisionHandler {
    fn default() -> Self {
        Self::new()
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
        _dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &ContactPair,
        _total_force_magnitude: Real,
    ) {
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Collision {
    handles: [ColliderHandle; 2],
}

impl Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline").finish_non_exhaustive()
    }
}

fn build_shape(shape: &ColliderShape) -> SharedShape {
    match shape {
        ColliderShape::Cuboid(cuboid) => SharedShape::cuboid(cuboid.hx, cuboid.hy, cuboid.hz),
        ColliderShape::Ball(ball) => SharedShape::ball(ball.radius),
        ColliderShape::Capsule(capsule) => match capsule.axis {
            Axis::X => SharedShape::capsule_x(capsule.half_height, capsule.radius),
            Axis::Y => SharedShape::capsule_y(capsule.half_height, capsule.radius),
            Axis::Z => SharedShape::capsule_z(capsule.half_height, capsule.radius),
        },
        ColliderShape::TriMesh(mesh) => {
            let vertices = mesh
                .vertices()
                .iter()
                .map(|vertex| OPoint::<f32, Const<3>>::new(vertex.x, vertex.y, vertex.z))
                .collect();
            let indices = mesh
                .indices()
                .windows(3)
                .map(|indices| indices.try_into().unwrap())
                .collect();

            SharedShape::trimesh(vertices, indices)
        }
    }
}

#[cfg(test)]
mod tests {
    use game_common::components::{
        Collider, ColliderShape, Cuboid, RigidBody, RigidBodyKind, Transform,
    };
    use game_common::events::EventQueue;
    use game_common::world::World;
    use glam::{Quat, Vec3};

    use crate::query::QueryFilter;
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

    #[test]
    fn pipeline_cast_shape_cuboid() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert_typed(
            entity,
            RigidBody {
                kind: RigidBodyKind::Fixed,
                linvel: Vec3::ZERO,
                angvel: Vec3::ZERO,
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

        let res = pipeline
            .cast_shape(
                Vec3::new(5.0, 0.0, 0.0),
                Quat::IDENTITY,
                Vec3::new(-1.0, 0.0, 0.0),
                6.0,
                &ColliderShape::Cuboid(Cuboid {
                    hx: 1.0,
                    hy: 1.0,
                    hz: 1.0,
                }),
                &QueryFilter::default(),
            )
            .unwrap();

        assert_eq!(res.entity, entity);
        assert_eq!(res.toi, 3.0);
    }
}
