use ahash::HashMap;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::events::{Event, EventQueue};
use game_common::world::entity::Entity;
use game_physics::data::{Collider, Cuboid};
use game_physics::PhysicsStateProvider;
use game_scene::scene2::{ColliderShape, Component};
use game_script::WorldProvider;
use glam::{Quat, Vec3};

use crate::SceneState;

use super::state::WorldState;

#[derive(Debug, Default)]
pub struct PhysicsState {
    pub bodies: Vec<EntityId>,
    pub colliders: HashMap<EntityId, Vec<(Transform, Collider)>>,
}

impl PhysicsState {
    pub fn update(&mut self, state: &mut SceneState) {
        for mut key in state.graph.iter_added() {
            let node = state.graph.get(key).unwrap();

            let mut collider = None;
            for component in &node.components {
                collider = match component {
                    Component::Collider(collider) => Some(Collider {
                        restitution: collider.restitution,
                        friction: collider.friction,
                        shape: match collider.shape {
                            ColliderShape::Cuboid(cuboid) => {
                                game_physics::data::ColliderShape::Cuboid(Cuboid {
                                    hx: cuboid.hx,
                                    hy: cuboid.hy,
                                    hz: cuboid.hz,
                                })
                            }
                            _ => todo!(),
                        },
                    }),
                    _ => continue,
                };
            }

            let Some(collider) = collider else {
                continue;
            };

            // Find the root entity.
            while let Some((parent, _)) = state.graph.parent(key) {
                key = parent;
            }

            let entity = state.entities.get(&key).unwrap();

            self.colliders
                .entry(*entity)
                .or_default()
                .push((node.transform, collider));

            if !self.bodies.contains(entity) {
                self.bodies.push(*entity);
            }
        }

        // TODO: Cleanup HEHE
    }
}

pub struct PhysicsContext<'a> {
    pub state: &'a PhysicsState,
    pub world: &'a mut WorldState,
    pub event_queue: &'a mut EventQueue,
}

impl<'a> PhysicsStateProvider for PhysicsContext<'a> {
    fn bodies(&self) -> &[EntityId] {
        &self.state.bodies
    }

    fn colliders(&self, entity: EntityId) -> Option<&[(Transform, Collider)]> {
        self.state.colliders.get(&entity).map(|vec| vec.as_slice())
    }

    fn get(&self, entity: EntityId) -> Option<&Entity> {
        self.world.get(entity)
    }

    fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    fn set_rotation(&mut self, entity: EntityId, rotation: Quat) {
        self.world.get_mut(entity).unwrap().transform.rotation = rotation;
    }

    fn set_translation(&mut self, entity: EntityId, translation: Vec3) {
        self.world.get_mut(entity).unwrap().transform.translation = translation;
    }
}
