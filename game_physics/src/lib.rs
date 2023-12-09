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
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::events::{self, Event};
use game_common::world::entity::{Entity, EntityBody};
use game_tracing::trace_span;
use glam::{Quat, Vec3};
use handle::HandleMap;
use nalgebra::Isometry;
use parking_lot::Mutex;
use rapier3d::prelude::{
    ActiveEvents, BroadPhase, CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet,
    CollisionEvent, ContactPair, EventHandler, ImpulseJointSet, IntegrationParameters,
    IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryFilter, QueryPipeline,
    Ray, RigidBodyBuilder, RigidBodyHandle, RigidBodySet, RigidBodyType, Vector,
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

    pub fn step<S>(&mut self, state: &mut S)
    where
        S: PhysicsStateProvider,
    {
        let _span = trace_span!("Pipeline::step").entered();

        self.update_state(state);

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

        self.emit_events(state);
        self.write_back(state);
    }

    fn update_state<S>(&mut self, state: &S)
    where
        S: PhysicsStateProvider,
    {
        let mut handles = self.body_handles.clone();

        for id in state.bodies() {
            let entity = state.get(*id).unwrap();

            let Some(handle) = handles.remove(*id) else {
                self.add_entity(entity, state);
                continue;
            };

            let body = self.bodies.get_mut(handle).unwrap();

            let translation = vector(entity.transform.translation);
            if *body.translation() != translation {
                body.set_translation(translation, true);
            }

            let rotation = rotation(entity.transform.rotation);
            if *body.rotation() != rotation {
                body.set_rotation(rotation, true);
            }
        }

        for handle in handles.iter() {
            let body = self
                .bodies
                .remove(
                    handle,
                    &mut self.islands,
                    &mut self.colliders,
                    &mut self.impulse_joints,
                    &mut self.multibody_joints,
                    true,
                )
                .unwrap();

            self.controllers.remove(&handle);
            self.body_handles.remove2(handle);

            for collider in body.colliders() {
                self.collider_handles.remove2(*collider);
            }
        }
    }

    fn add_entity<S>(&mut self, entity: &Entity, state: &S)
    where
        S: PhysicsStateProvider,
    {
        let colliders = state.colliders(entity.id).unwrap();

        let body_type = match entity.body {
            EntityBody::Terrain(_) => RigidBodyType::Fixed,
            EntityBody::Object(_) => RigidBodyType::Fixed,
            EntityBody::Actor(_) => RigidBodyType::Dynamic,
            EntityBody::Item(_) => RigidBodyType::Dynamic,
        };

        let body = RigidBodyBuilder::new(body_type)
            .position(Isometry {
                // TODO: Should use inferred cell for terrain entities.
                translation: vector(entity.transform.translation).into(),
                rotation: rotation(entity.transform.rotation),
            })
            .ccd_enabled(true)
            .build();

        let body_handle = self.bodies.insert(body);
        self.body_handles.insert(entity.id, body_handle);

        for (transform, collider) in colliders {
            let mut builder = match collider.shape {
                crate::data::ColliderShape::Cuboid(cuboid) => {
                    ColliderBuilder::cuboid(cuboid.hx, cuboid.hy, cuboid.hz)
                }
            };

            builder = builder.position(Isometry {
                translation: vector(transform.translation).into(),
                rotation: rotation(transform.rotation),
            });
            builder = builder.active_events(ActiveEvents::COLLISION_EVENTS);

            let collider_handle =
                self.colliders
                    .insert_with_parent(builder.build(), body_handle, &mut self.bodies);

            self.collider_handles.insert(entity.id, collider_handle);
        }
    }

    fn write_back<S>(&mut self, state: &mut S)
    where
        S: PhysicsStateProvider,
    {
        for (handle, body) in self.bodies.iter() {
            let id = self.body_handles.get2(handle).unwrap();
            state.set_translation(id, vec3(*body.translation()));
            state.set_rotation(id, quat(*body.rotation()));
        }
    }

    fn emit_events<S>(&mut self, state: &mut S)
    where
        S: PhysicsStateProvider,
    {
        let events = self.event_handler.events.get_mut();

        for event in &*events {
            let lhs = self.collider_handles.get2(event.handles[0]).unwrap();
            let rhs = self.collider_handles.get2(event.handles[1]).unwrap();

            state.push_event(Event::Collision(events::CollisionEvent {
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
                dbg!(entity, toi);
                Some((entity, toi))
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

pub trait PhysicsStateProvider {
    fn get(&self, entity: EntityId) -> Option<&Entity>;
    /// Returns all rigid bodies.
    fn bodies(&self) -> &[EntityId];

    fn colliders(&self, entity: EntityId) -> Option<&[(Transform, crate::data::Collider)]>;

    fn push_event(&mut self, event: Event);

    fn set_translation(&mut self, entity: EntityId, translation: Vec3);
    fn set_rotation(&mut self, entity: EntityId, rotation: Quat);
}
