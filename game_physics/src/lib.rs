mod control;
mod convert;
mod handle;
mod pipeline;

use std::time::{Duration, Instant};

use bevy_ecs::system::Resource;
use control::CharacterController;
use convert::{point, quat, rotation, vec3, vector};
use game_common::events::{self, Event, EventQueue};
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::world::WorldState;
use glam::{Quat, Vec3};
use handle::HandleMap;
use nalgebra::Isometry;
use parking_lot::Mutex;
use rapier3d::prelude::{
    ActiveEvents, BroadPhase, CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet,
    CollisionEvent, ContactPair, EventHandler, ImpulseJointSet, IntegrationParameters,
    IslandManager, LockedAxes, MultibodyJointSet, NarrowPhase, PhysicsPipeline, RigidBodyBuilder,
    RigidBodyHandle, RigidBodySet, RigidBodyType, Vector,
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

    /// When the pipeline is called for the first time, all data needs to be loaded from the world.
    /// The pipeline can go over to a event-driven mechanism after that.
    is_initialized: bool,

    body_handles: HandleMap<RigidBodyHandle>,
    // We need the collider for collision events.
    collider_handles: HandleMap<ColliderHandle>,

    last_timestep: Instant,

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
            is_initialized: false,
            body_handles: HandleMap::new(),
            last_timestep: Instant::now(),
            event_handler: CollisionHandler::new(),
            collider_handles: HandleMap::new(),
        }
    }

    pub fn step(&mut self, world: &mut WorldState, events: &mut EventQueue) {
        if !self.is_initialized {
            self.prepare_init(world);
        } else {
            self.prepare_poll(world);
        }

        let mut steps = 0;

        let now = Instant::now();
        while self.last_timestep < now {
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

            self.last_timestep += Duration::from_secs_f64(1.0 / 60.0);
            steps += 1;
        }

        tracing::trace!("stepping physics for {} steps", steps);

        self.emit_events(events);

        self.write_back(world);
    }

    fn prepare_init(&mut self, world: &mut WorldState) {
        let Some(view) = world.back() else {
            return;
        };

        for entity in view.iter() {
            self.add_entity(entity);
        }

        self.is_initialized = true;
    }

    fn prepare_poll(&mut self, world: &mut WorldState) {
        let Some(view) = world.back() else {
            return;
        };

        for event in view.deltas() {
            match event {
                EntityChange::Create { entity } => {
                    self.add_entity(entity);
                }
                EntityChange::Translate { id, translation } => {
                    if let Some(handle) = self.body_handles.get(*id) {
                        let body = self.bodies.get_mut(handle).unwrap();
                        body.set_translation(vector(*translation), true);
                    } else {
                        tracing::warn!("invalid entity {:?}", id)
                    }
                }
                EntityChange::Rotate { id, rotation: rot } => {
                    if let Some(handle) = self.body_handles.get(*id) {
                        let body = self.bodies.get_mut(handle).unwrap();
                        body.set_rotation(rotation(*rot), true);
                    } else {
                        tracing::warn!("invalid entity {:?}", id);
                    }
                }
                EntityChange::Destroy { id } => {
                    if let Some(handle) = self.body_handles.remove(*id) {
                        self.bodies.remove(
                            handle,
                            &mut self.islands,
                            &mut self.colliders,
                            &mut self.impulse_joints,
                            &mut self.multibody_joints,
                            true,
                        );
                    } else {
                        tracing::warn!("invalid entity {:?}", id);
                    }
                }
                _ => (),
            }
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

                (body_handle, col_handle)
            }
            EntityBody::Item(item) => {
                todo!()
            }
        };

        self.body_handles.insert(entity.id, body);
        self.collider_handles.insert(entity.id, collider);
    }

    fn write_back(&mut self, world: &mut WorldState) {
        let Some(mut view) = world.back_mut() else {
            return;
        };

        for (handle, body) in self.bodies.iter() {
            if body.is_sleeping() {
                continue;
            }

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
