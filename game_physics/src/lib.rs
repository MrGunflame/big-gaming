use std::time::{Duration, Instant};

use bevy_ecs::system::Resource;
use game_common::math::RotationExt;
use game_common::world::entity::{Entity, EntityBody};
use game_common::world::snapshot::EntityChange;
use game_common::world::world::WorldState;
use handle::HandleMap;
use rapier3d::prelude::{
    BroadPhase, CCDSolver, ColliderBuilder, ColliderSet, ImpulseJointSet, IntegrationParameters,
    IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, RigidBodyBuilder,
    RigidBodyHandle, RigidBodySet, RigidBodyType, Vector,
};

mod handle;
mod pipeline;

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

    last_timestep: Instant,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector::new(0.0, -9.81, 0.0),
            integration_parameters: IntegrationParameters::default(),
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
        }
    }

    pub fn step(&mut self, world: &mut WorldState) {
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
                &(),
            );

            self.last_timestep += Duration::from_secs_f64(1.0 / 60.0);
            steps += 1;
        }

        tracing::info!("stepping physics for {} steps", steps);

        self.write_back(world);
    }

    fn prepare_init(&mut self, world: &mut WorldState) {
        let Some(view) = world.front() else {
            return;
        };

        for entity in view.iter() {
            self.add_entity(entity);
        }

        self.is_initialized = true;
    }

    fn prepare_poll(&mut self, world: &mut WorldState) {
        let Some(view) = world.front() else {
            return;
        };

        for event in view.deltas() {
            match event {
                EntityChange::Create { entity } => {
                    self.add_entity(entity);
                }
                EntityChange::Translate {
                    id,
                    translation,
                    cell: _,
                } => {
                    if let Some(handle) = self.body_handles.get(*id) {
                        let body = self.bodies.get_mut(handle).unwrap();
                        body.set_translation((*translation).into(), true);
                    } else {
                        tracing::warn!("invalid entity {:?}", id)
                    }
                }
                EntityChange::Rotate { id, rotation } => {
                    if let Some(handle) = self.body_handles.get(*id) {
                        let body = self.bodies.get_mut(handle).unwrap();
                        body.set_rotation((*rotation).into(), true);
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
        let handle = match &entity.body {
            // Terrain can never move.
            EntityBody::Terrain(terrain) => {
                let body = RigidBodyBuilder::new(RigidBodyType::Fixed)
                    .translation(terrain.cell.min().into())
                    // .rotation(Vector::new(0.0, 0.0, -1.0))
                    .ccd_enabled(true)
                    .build();

                let body_handle = self.bodies.insert(body);
                self.body_handles.insert(entity.id, body_handle);

                let (vertices, indices) = terrain.verts_indices();
                let vertices = vertices.into_iter().map(|vert| vert.into()).collect();

                let collider = ColliderBuilder::trimesh(vertices, indices);
                self.colliders
                    .insert_with_parent(collider, body_handle, &mut self.bodies);

                body_handle
            }
            _ => {
                let body = RigidBodyBuilder::new(RigidBodyType::Dynamic)
                    .translation(entity.transform.translation.into())
                    .rotation(entity.transform.rotation.dir_vec().into())
                    .ccd_enabled(true)
                    .lock_rotations()
                    .build();

                let body_handle = self.bodies.insert(body);
                let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0);
                self.colliders
                    .insert_with_parent(collider, body_handle, &mut self.bodies);

                body_handle
            }
        };

        self.body_handles.insert(entity.id, handle);
    }

    fn write_back(&mut self, world: &mut WorldState) {
        let Some(mut view) = world.front_mut() else {
            return;
        };

        for (handle, body) in self.bodies.iter() {
            if body.is_sleeping() {
                continue;
            }

            let id = self.body_handles.get2(handle).unwrap();
            if let Some(mut entity) = view.get_mut(id) {
                entity.transform.translation = (*body.translation()).into();
                entity.transform.rotation = (*body.rotation()).into();
            }
        }
    }
}
