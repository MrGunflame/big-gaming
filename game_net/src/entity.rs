//! Entity translations

use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::net::ServerEntity;

use crate::proto::{
    EntityCreate, EntityDestroy, EntityRotate, EntityTranslate, EntityVelocity, Frame, PlayerJoin,
    PlayerLeave, SpawnHost,
};
use crate::snapshot::Command;

#[derive(Clone, Debug, Default)]
pub struct Entities {
    host: HashMap<EntityId, ServerEntity>,
    remote: HashMap<ServerEntity, EntityId>,
    next_server_id: u64,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            host: HashMap::new(),
            remote: HashMap::new(),
            next_server_id: 0,
        }
    }

    pub fn insert(&mut self, local: EntityId, remote: ServerEntity) {
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

    /// Unpacks a raw [`Frame`] into a game [`Command`].
    pub fn unpack(&mut self, frame: Frame) -> Option<Command> {
        match frame {
            Frame::EntityCreate(frame) => {
                let id = EntityId::new();
                self.insert(id, frame.entity);

                Some(Command::EntityCreate {
                    id,
                    kind: frame.kind,
                    translation: frame.translation,
                    rotation: frame.rotation,
                })
            }
            Frame::EntityDestroy(frame) => {
                let id = self.get(frame.entity)?;
                self.remove(id);

                Some(Command::EntityDestroy { id })
            }
            Frame::EntityTranslate(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityTranslate {
                    id,
                    translation: frame.translation,
                })
            }
            Frame::EntityRotate(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityRotate {
                    id,
                    rotation: frame.rotation,
                })
            }
            Frame::EntityVelocity(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityVelocity {
                    id,
                    linvel: frame.linvel,
                    angvel: frame.angvel,
                })
            }
            Frame::SpawnHost(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::SpawnHost { id })
            }
            Frame::PlayerJoin(_) => Some(Command::PlayerJoin),
            Frame::PlayerLeave(_) => Some(Command::PlayerLeave),
        }
    }

    pub fn pack(&mut self, cmd: &Command) -> Option<Frame> {
        match cmd {
            Command::EntityCreate {
                id,
                kind,
                translation,
                rotation,
            } => {
                let entity = self.new_id();
                self.insert(*id, entity);

                dbg!(id, entity);

                Some(Frame::EntityCreate(EntityCreate {
                    entity,
                    translation: *translation,
                    rotation: *rotation,
                    kind: *kind,
                }))
            }
            Command::EntityDestroy { id } => {
                let id = self.get(*id)?;

                Some(Frame::EntityDestroy(EntityDestroy { entity: id }))
            }
            Command::EntityTranslate { id, translation } => {
                let id = self.get(*id)?;

                Some(Frame::EntityTranslate(EntityTranslate {
                    entity: id,
                    translation: *translation,
                }))
            }
            Command::EntityRotate { id, rotation } => {
                let id = self.get(*id)?;

                Some(Frame::EntityRotate(EntityRotate {
                    entity: id,
                    rotation: *rotation,
                }))
            }
            Command::EntityVelocity { id, linvel, angvel } => {
                let id = self.get(*id)?;

                Some(Frame::EntityVelocity(EntityVelocity {
                    entity: id,
                    linvel: *linvel,
                    angvel: *angvel,
                }))
            }
            Command::PlayerJoin => Some(Frame::PlayerJoin(PlayerJoin {})),
            Command::PlayerLeave => Some(Frame::PlayerLeave(PlayerLeave {})),
            Command::SpawnHost { id } => {
                let id = self.get(*id)?;

                Some(Frame::SpawnHost(SpawnHost { entity: id }))
            }
        }
    }

    fn new_id(&mut self) -> ServerEntity {
        let id = self.next_server_id;
        self.next_server_id += 1;
        ServerEntity(id)
    }
}

pub trait ServerEntityTranslation: private::Sealed {
    type Target;

    #[doc(hidden)]
    fn get(self, entities: &Entities) -> Option<Self::Target>;

    #[doc(hidden)]
    fn remove(self, entities: &mut Entities) -> Option<Self::Target>;
}

impl ServerEntityTranslation for EntityId {
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
    type Target = EntityId;

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
impl private::Sealed for EntityId {}

#[doc(hidden)]
impl private::Sealed for ServerEntity {}

mod private {
    pub trait Sealed {}
}
