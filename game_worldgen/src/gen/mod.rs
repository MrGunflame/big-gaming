use game_common::components::{
    Collider, ColliderShape, Cuboid, MeshInstance, RigidBody, RigidBodyKind, Transform,
};
use game_common::world::gen::{CellBuilder, EntityBuilder, Generate};
use game_common::world::CellId;
use glam::{Quat, Vec3};

use crate::{Component, Entity};

pub struct StaticGenerator {
    pub(crate) entities: Vec<Entity>,
}

impl Generate for StaticGenerator {
    fn generate(&self, cell: &mut CellBuilder) {
        for entity in &self.entities {
            if CellId::from(Vec3::from_array(entity.transform.translation)) != cell.id() {
                continue;
            }

            let mut builder = EntityBuilder::new().transform(Transform {
                translation: Vec3::from_array(entity.transform.translation),
                rotation: Quat::from_array(entity.transform.rotation),
                scale: Vec3::from_array(entity.transform.scale),
            });

            for component in &entity.components {
                match component {
                    Component::Collider(collider) => {
                        builder = builder.component_typed(Collider {
                            friction: collider.friction,
                            restitution: collider.restitution,
                            shape: match collider.shape {
                                crate::ColliderShape::Cuboid(cuboid) => {
                                    ColliderShape::Cuboid(Cuboid {
                                        hx: cuboid.x,
                                        hy: cuboid.y,
                                        hz: cuboid.z,
                                    })
                                }
                            },
                        });
                    }
                    Component::MeshInstance(string) => {
                        builder = builder.component_typed(MeshInstance {
                            path: string.clone(),
                        });
                    }
                    Component::RigidBody(body) => {
                        builder = builder.component_typed(RigidBody {
                            kind: match body.kind {
                                crate::RigidBodyKind::Fixed => RigidBodyKind::Fixed,
                                crate::RigidBodyKind::Dynamic => RigidBodyKind::Dynamic,
                            },
                            angvel: Vec3::from_array(body.angvel),
                            linvel: Vec3::from_array(body.linvel),
                        });
                    }
                }
            }

            cell.spawn(builder);
        }
    }
}
