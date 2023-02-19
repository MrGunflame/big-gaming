//! Entity translations

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use game_common::net::ServerEntity;

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
