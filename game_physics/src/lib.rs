#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_crate_dependencies)]

mod control;
mod convert;
mod handle;
mod pipeline;
pub mod query;

use std::collections::HashMap;

use bevy_ecs::system::Resource;
use control::CharacterController;
use convert::{point, quat, rotation, vec3, vector};
use game_common::entity::EntityId;
use game_common::events::{self, Event, EventQueue};
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::world::{AsView, WorldState, WorldViewMut};
use glam::{Quat, Vec3};
use handle::HandleMap;
use nalgebra::Isometry;
use parking_lot::Mutex;
use rapier3d::prelude::{
    ActiveEvents, BroadPhase, CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet,
    CollisionEvent, ContactPair, EventHandler, ImpulseJointSet, IntegrationParameters,
    IslandManager, LockedAxes, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryFilter,
    QueryPipeline, Ray, RigidBodyBuilder, RigidBodyHandle, RigidBodySet, RigidBodyType, Vector,
};

#[derive(Resource)]
pub struct Pipeline {
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
    body_handles: HandleMap<RigidBodyHandle>,
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

    pub fn step(
        &mut self,
        world: &mut WorldState,
        mut start: ControlFrame,
        end: ControlFrame,
        events: &mut EventQueue,
    ) {
        let mut steps = 0;

        while start <= end {
            let mut view = world.get_mut(start).unwrap();
            self.read_snapshot(&view);

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

            self.write_snapshot(&mut view);

            steps += 1;

            start += 1;
        }

        tracing::trace!("stepping physics for {} steps", steps);
    }

    fn read_snapshot<V>(&mut self, view: V)
    where
        V: AsView,
    {
        self.islands = IslandManager::new();
        self.broad_phase = BroadPhase::new();
        self.narrow_phase = NarrowPhase::new();
        self.colliders = ColliderSet::new();
        self.collider_handles = HandleMap::new();
        self.bodies = RigidBodySet::new();
        self.body_handles = HandleMap::new();
        self.query_pipeline = QueryPipeline::new();
        self.impulse_joints = ImpulseJointSet::new();
        self.multibody_joints = MultibodyJointSet::new();
        self.controllers = HashMap::new();
        self.ccd_solver = CCDSolver::new();

        for entity in view.iter() {
            self.add_entity(entity);
        }
    }

    fn add_entity(&mut self, entity: &Entity) {
        let (body, collider) = match &entity.body {
            // Terrain can never move.
            EntityBody::Terrain(terrain) => {
                let body = RigidBodyBuilder::new(RigidBodyType::Fixed)
                    .position(Isometry {
                        translation: vector(terrain.mesh.cell.min()).into(),
                        rotation: rotation(Quat::IDENTITY),
                    })
                    .ccd_enabled(true)
                    .build();

                let body_handle = self.bodies.insert(body);
                self.body_handles.insert(entity.id, body_handle);

                let (vertices, indices) = terrain.mesh.verts_indices();
                let vertices = vertices.into_iter().map(|vert| point(vert)).collect();

                let collider = ColliderBuilder::trimesh(vertices, indices)
                    .active_events(ActiveEvents::COLLISION_EVENTS);
                let col_handle =
                    self.colliders
                        .insert_with_parent(collider, body_handle, &mut self.bodies);

                (body_handle, col_handle)
            }
            EntityBody::Object(object) => {
                let body = RigidBodyBuilder::new(RigidBodyType::Fixed)
                    .position(Isometry {
                        translation: vector(entity.transform.translation).into(),
                        rotation: rotation(entity.transform.rotation),
                    })
                    .ccd_enabled(true)
                    .build();

                let body_handle = self.bodies.insert(body);
                let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                    .active_events(ActiveEvents::COLLISION_EVENTS);

                let col_handle =
                    self.colliders
                        .insert_with_parent(collider, body_handle, &mut self.bodies);

                (body_handle, col_handle)
            }
            EntityBody::Actor(actor) => {
                let body = RigidBodyBuilder::new(RigidBodyType::KinematicVelocityBased)
                    .position(Isometry {
                        translation: vector(entity.transform.translation).into(),
                        rotation: rotation(extract_actor_rotation(entity.transform.rotation)),
                    })
                    .ccd_enabled(true)
                    .lock_rotations()
                    .locked_axes(LockedAxes::ROTATION_LOCKED)
                    .build();

                let body_handle = self.bodies.insert(body);
                let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                    .active_events(ActiveEvents::COLLISION_EVENTS);
                let col_handle =
                    self.colliders
                        .insert_with_parent(collider, body_handle, &mut self.bodies);

                self.controllers.insert(body_handle, CharacterController {});

                (body_handle, col_handle)
            }
            EntityBody::Item(item) => {
                let body = RigidBodyBuilder::new(RigidBodyType::Dynamic)
                    .position(Isometry {
                        translation: vector(entity.transform.translation).into(),
                        rotation: rotation(entity.transform.rotation),
                    })
                    .ccd_enabled(true)
                    .build();

                let body_handle = self.bodies.insert(body);
                let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                    .active_events(ActiveEvents::COLLISION_EVENTS);

                let col_handle =
                    self.colliders
                        .insert_with_parent(collider, body_handle, &mut self.bodies);

                (body_handle, col_handle)
            }
        };

        self.body_handles.insert(entity.id, body);
        self.collider_handles.insert(entity.id, collider);
    }

    fn write_snapshot(&mut self, view: &mut WorldViewMut<'_>) {
        for (handle, body) in self.bodies.iter() {
            let id = self.body_handles.get2(handle).unwrap();
            if let Some(mut entity) = view.get_mut(id) {
                entity.transform.translation = vec3(*body.translation());
                entity.transform.rotation = quat(*body.rotation());
            }
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
