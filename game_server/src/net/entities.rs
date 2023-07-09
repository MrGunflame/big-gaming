use ahash::HashMap;
use game_common::entity::EntityId;
use game_common::net::ServerEntity;

#[derive(Clone, Debug, Default)]
pub struct Entities {
    host: HashMap<EntityId, ServerEntity>,
    remote: HashMap<ServerEntity, EntityId>,
    next_id: u64,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            host: HashMap::default(),
            remote: HashMap::default(),
            next_id: 0,
        }
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

    pub fn insert(&mut self, local: EntityId) -> ServerEntity {
        let id = ServerEntity(self.next_id);
        self.next_id += 1;

        self.host.insert(local, id);
        self.remote.insert(id, local);
        id
    }
}

pub trait ServerEntityTranslation {
    type Target;

    fn get(self, entities: &Entities) -> Option<Self::Target>;

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
