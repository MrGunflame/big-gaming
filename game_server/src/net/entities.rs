use ahash::HashMap;
use game_common::entity::EntityId;
use game_common::net::ServerEntity;

#[derive(Clone, Debug, Default)]
pub struct Entities {
    server: HashMap<EntityId, ServerEntity>,
    client: HashMap<ServerEntity, EntityId>,
    next_id: u64,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            server: HashMap::default(),
            client: HashMap::default(),
            next_id: 0,
        }
    }

    pub fn contains<E>(&self, entity: E) -> bool
    where
        E: ServerEntityTranslation,
    {
        entity.get(self).is_some()
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
        debug_assert_eq!(self.server.len(), self.client.len());
        self.server.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, local: EntityId) -> ServerEntity {
        let id = ServerEntity(self.next_id);
        self.next_id += 1;

        self.server.insert(local, id);
        self.client.insert(id, local);
        id
    }

    pub fn clear(&mut self) {
        self.server.clear();
        self.client.clear();
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
        entities.server.get(&self).copied()
    }

    #[inline]
    fn remove(self, entities: &mut Entities) -> Option<Self::Target> {
        let id = entities.server.remove(&self)?;
        entities.client.remove(&id);
        Some(id)
    }
}

impl ServerEntityTranslation for ServerEntity {
    type Target = EntityId;

    #[inline]
    fn get(self, entities: &Entities) -> Option<Self::Target> {
        entities.client.get(&self).copied()
    }

    #[inline]
    fn remove(self, entities: &mut Entities) -> Option<Self::Target> {
        let id = entities.client.remove(&self)?;
        entities.server.remove(&id);
        Some(id)
    }
}
