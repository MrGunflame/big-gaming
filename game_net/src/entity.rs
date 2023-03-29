//! Entity translations

use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::net::ServerEntity;

use crate::proto::{
    EntityCreate, EntityDestroy, EntityHealth, EntityRotate, EntityTranslate, EntityVelocity,
    Frame, SpawnHost, Terrain,
};
use crate::snapshot::Command;

#[derive(Clone, Debug, Default)]
pub struct Entities {
    host: HashMap<EntityId, ServerEntity>,
    remote: HashMap<ServerEntity, EntityId>,
    next_server_id: u64,
    next_id: u64,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            host: HashMap::new(),
            remote: HashMap::new(),
            next_server_id: 0,
            next_id: 0,
        }
    }

    pub fn insert(&mut self, local: EntityId, remote: ServerEntity) {
        if cfg!(debug_assertions) {
            assert!(!self.host.contains_key(&local));
            assert!(!self.remote.contains_key(&remote));
        }

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

    pub fn len(&self) -> usize {
        debug_assert_eq!(self.host.len(), self.remote.len());

        self.host.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Unpacks a raw [`Frame`] into a game [`Command`].
    pub fn unpack(&mut self, frame: Frame) -> Option<Command> {
        match frame {
            Frame::EntityCreate(frame) => {
                let id = EntityId::from_raw(self.next_id);
                self.next_id += 1;

                self.insert(id, frame.entity);

                Some(Command::EntityCreate {
                    id,
                    translation: frame.translation,
                    rotation: frame.rotation,
                    data: frame.data,
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
            Frame::EntityHealth(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::EntityHealth {
                    id,
                    health: frame.health,
                })
            }
            Frame::SpawnHost(frame) => {
                let id = self.get(frame.entity)?;

                Some(Command::SpawnHost { id })
            }
            Frame::WorldTerrain(frame) => Some(Command::WorldTerrain {
                cell: frame.cell,
                height: frame.height,
            }),
        }
    }

    pub fn pack(&mut self, cmd: &Command) -> Option<Frame> {
        match cmd {
            Command::EntityCreate {
                id,
                translation,
                rotation,
                data,
            } => {
                let entity = self.new_id();
                self.insert(*id, entity);

                Some(Frame::EntityCreate(EntityCreate {
                    entity,
                    translation: *translation,
                    rotation: *rotation,
                    data: data.clone(),
                }))
            }
            Command::EntityDestroy { id } => {
                let id = self.remove(*id)?;

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
            Command::EntityHealth { id, health } => {
                let entity = self.get(*id)?;

                Some(Frame::EntityHealth(EntityHealth {
                    entity,
                    health: *health,
                }))
            }
            Command::Connected => None,
            Command::Disconnected => None,
            Command::SpawnHost { id } => {
                let id = self.get(*id)?;

                Some(Frame::SpawnHost(SpawnHost { entity: id }))
            }
            Command::WorldTerrain { cell, height } => Some(Frame::WorldTerrain(Terrain {
                cell: *cell,
                height: height.clone(),
            })),
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

#[cfg(test)]
mod tests {
    use super::{Entities, EntityId, ServerEntity};

    #[test]
    fn test_entities() {
        let mut entities = Entities::new();
        assert_eq!(entities.len(), 0);
        assert_eq!(entities.is_empty(), true);

        entities.insert(EntityId::from_raw(0), ServerEntity(0));
        assert_eq!(entities.get(EntityId::from_raw(0)), Some(ServerEntity(0)));
        assert_eq!(entities.get(ServerEntity(0)), Some(EntityId::from_raw(0)));
        assert_eq!(entities.len(), 1);

        entities.insert(EntityId::from_raw(1), ServerEntity(1));
        assert_eq!(entities.get(EntityId::from_raw(1)), Some(ServerEntity(1)));
        assert_eq!(entities.get(ServerEntity(1)), Some(EntityId::from_raw(1)));
        assert_eq!(entities.len(), 2);

        assert_eq!(entities.get(EntityId::from_raw(0)), Some(ServerEntity(0)));
        assert_eq!(entities.get(ServerEntity(0)), Some(EntityId::from_raw(0)));

        assert_eq!(
            entities.remove(EntityId::from_raw(0)),
            Some(ServerEntity(0))
        );
        assert_eq!(entities.get(EntityId::from_raw(0)), None);
        assert_eq!(entities.get(ServerEntity(0)), None);
        assert_eq!(entities.len(), 1);

        assert_eq!(
            entities.remove(ServerEntity(1)),
            Some(EntityId::from_raw(1))
        );
        assert_eq!(entities.get(EntityId::from_raw(1)), None);
        assert_eq!(entities.get(ServerEntity(1)), None);
        assert_eq!(entities.len(), 0);
    }
}
