//! Entity translations

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use game_common::net::ServerEntity;

use crate::proto::{EntityCreate, EntityDestroy, EntityKind, EntityRotate, EntityTranslate, Frame};
use crate::snapshot::Command;

#[derive(Clone, Debug, Default)]
pub struct Entities {
    host: HashMap<Entity, ServerEntity>,
    remote: HashMap<ServerEntity, Entity>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            host: HashMap::new(),
            remote: HashMap::new(),
        }
    }

    pub fn insert(&mut self, local: Entity, remote: ServerEntity) {
        self.host.insert(local, remote);
        self.remote.insert(remote, local);
    }

    pub fn remove<E>(&mut self, entity: E) -> Option<E::Target>
    where
        E: ServerEntityTranslation,
    {
        entity.remove(self)
    }

    pub fn get<E>(&self, entity: E) -> Option<E::Target>
    where
        E: ServerEntityTranslation,
    {
        entity.get(self)
    }

    pub fn translate(&self, frame: Frame) -> Option<Command> {
        match frame {
            Frame::EntityCreate(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityCreate {
                    id,
                    translation: frame.translation,
                    rotation: frame.rotation,
                })
            }
            Frame::EntityDestroy(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityDestroy { id })
            }
            Frame::EntityTranslate(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityTranslate {
                    id,
                    translation: frame.translation,
                })
            }
            _ => unimplemented!(),
        }
    }

    pub fn translate_cmd(&self, cmd: Command) -> Option<Frame> {
        match cmd {
            Command::EntityCreate {
                id,
                translation,
                rotation,
            } => {
                let id = self.get(id)?;

                Some(Frame::EntityCreate(EntityCreate {
                    entity: id,
                    translation,
                    rotation,
                    kind: EntityKind::Object,
                }))
            }
            Command::EntityDestroy { id } => {
                let id = self.get(id)?;

                Some(Frame::EntityDestroy(EntityDestroy { entity: id }))
            }
            Command::EntityTranslate { id, translation } => {
                let id = self.get(id)?;

                Some(Frame::EntityTranslate(EntityTranslate {
                    entity: id,
                    translation,
                }))
            }
            Command::EntityRotate { id, rotation } => {
                let id = self.get(id)?;

                Some(Frame::EntityRotate(EntityRotate {
                    entity: id,
                    rotation,
                }))
            }
            Command::PlayerJoin => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

pub trait ServerEntityTranslation: private::Sealed {
    type Target;

    #[doc(hidden)]
    fn get(self, entities: &Entities) -> Option<Self::Target>;

    #[doc(hidden)]
    fn remove(self, entities: &mut Entities) -> Option<Self::Target>;
}

impl ServerEntityTranslation for Entity {
    type Target = ServerEntity;

    #[inline]
    fn get(self, entities: &Entities) -> Option<Self::Target> {
        entities.host.get(&self).copied()
    }

    #[inline]
    fn remove(self, entities: &mut Entities) -> Option<Self::Target> {
        let id = entities.host.remove(&self)?;
        entities.remote.remove(&id);
        Some(id)
    }
}

impl ServerEntityTranslation for ServerEntity {
    type Target = Entity;

    #[inline]
    fn get(self, entities: &Entities) -> Option<Self::Target> {
        entities.remote.get(&self).copied()
    }

    #[inline]
    fn remove(self, entities: &mut Entities) -> Option<Self::Target> {
        let id = entities.remote.remove(&self)?;
        entities.host.remove(&id);
        Some(id)
    }
}

#[doc(hidden)]
impl private::Sealed for Entity {}

#[doc(hidden)]
impl private::Sealed for ServerEntity {}

mod private {
    pub trait Sealed {}
}
