//! Entity translations

use std::collections::HashMap;

use game_common::entity::EntityId;
use game_common::net::ServerEntity;

use crate::proto::{
    EntityAction, EntityCreate, EntityDestroy, EntityHealth, EntityRotate, EntityTranslate, Frame,
    InventoryItemAdd, InventoryItemRemove, InventoryItemUpdate, SpawnHost,
};
use crate::snapshot::Command;

use crate::snapshot::{
    EntityAction as EntityActionCommand, EntityCreate as EntityCreateCommand,
    EntityDestroy as EntityDestroyCommand, EntityHealth as EntityHealthCommand,
    EntityRotate as EntityRotateCommand, EntityTranslate as EntityTranslateCommand,
    InventoryItemAdd as InventoryItemAddCommand, InventoryItemRemove as InventoryItemRemoveCommand,
    InventoryUpdate as InventoryUpdateCommand, SpawnHost as SpawnHostCommand,
};

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
        let id = ServerEntity(self.next_server_id);
        self.next_server_id += 1;

        self.host.insert(local, id);
        self.remote.insert(id, local);
        id
    }

    pub fn insert_client(&mut self, local: EntityId, remote: ServerEntity) {
        self.host.insert(local, remote);
        self.remote.insert(remote, local);
    }
}

pub(crate) fn unpack_command(frame: Frame) -> Option<Command> {
    match frame {
        Frame::EntityCreate(frame) => Some(Command::EntityCreate(EntityCreateCommand {
            id: frame.entity,
            translation: frame.translation,
            rotation: frame.rotation,
            data: frame.data,
        })),
        Frame::EntityDestroy(frame) => Some(Command::EntityDestroy(EntityDestroyCommand {
            id: frame.entity,
        })),
        Frame::EntityTranslate(frame) => Some(Command::EntityTranslate(EntityTranslateCommand {
            id: frame.entity,
            translation: frame.translation,
        })),
        Frame::EntityRotate(frame) => Some(Command::EntityRotate(EntityRotateCommand {
            id: frame.entity,
            rotation: frame.rotation,
        })),
        Frame::EntityHealth(frame) => Some(Command::EntityHealth(EntityHealthCommand {
            id: frame.entity,
            health: frame.health,
        })),
        Frame::EntityAction(frame) => Some(Command::EntityAction(EntityActionCommand {
            id: frame.entity,
            action: frame.action,
        })),
        Frame::SpawnHost(frame) => Some(Command::SpawnHost(SpawnHostCommand { id: frame.entity })),
        Frame::InventoryItemAdd(frame) => {
            Some(Command::InventoryItemAdd(InventoryItemAddCommand {
                entity: frame.entity,
                slot: frame.id,
                item: frame.item,
            }))
        }
        Frame::InventoryItemRemove(frame) => {
            Some(Command::InventoryItemRemove(InventoryItemRemoveCommand {
                entity: frame.entity,
                slot: frame.id,
            }))
        }
        Frame::InventoryItemUpdate(frame) => {
            Some(Command::InventoryUpdate(InventoryUpdateCommand {
                entity: frame.entity,
                slot: frame.id,
                equipped: frame.equipped,
                hidden: frame.hidden,
            }))
        }
    }
}

pub(crate) fn pack_command(cmd: &Command) -> Option<Frame> {
    match cmd {
        Command::EntityCreate(cmd) => Some(Frame::EntityCreate(EntityCreate {
            entity: cmd.id,
            translation: cmd.translation,
            rotation: cmd.rotation,
            data: cmd.data.clone(),
        })),
        Command::EntityDestroy(cmd) => Some(Frame::EntityDestroy(EntityDestroy { entity: cmd.id })),
        Command::EntityTranslate(cmd) => Some(Frame::EntityTranslate(EntityTranslate {
            entity: cmd.id,
            translation: cmd.translation,
        })),
        Command::EntityRotate(cmd) => Some(Frame::EntityRotate(EntityRotate {
            entity: cmd.id,
            rotation: cmd.rotation,
        })),
        Command::EntityHealth(cmd) => Some(Frame::EntityHealth(EntityHealth {
            entity: cmd.id,
            health: cmd.health,
        })),
        Command::EntityAction(cmd) => Some(Frame::EntityAction(EntityAction {
            entity: cmd.id,
            action: cmd.action,
        })),
        Command::SpawnHost(cmd) => Some(Frame::SpawnHost(SpawnHost { entity: cmd.id })),
        Command::InventoryItemAdd(cmd) => Some(Frame::InventoryItemAdd(InventoryItemAdd {
            entity: cmd.entity,
            id: cmd.slot,
            item: cmd.item,
        })),
        Command::InventoryItemRemove(cmd) => {
            Some(Frame::InventoryItemRemove(InventoryItemRemove {
                entity: cmd.entity,
                id: cmd.slot,
            }))
        }
        Command::InventoryUpdate(cmd) => Some(Frame::InventoryItemUpdate(InventoryItemUpdate {
            entity: cmd.entity,
            id: cmd.slot,
            equipped: cmd.equipped,
            hidden: cmd.hidden,
        })),
        Command::Connected(_) => None,
        Command::Disconnected => None,
        Command::ReceivedCommands(_) => None,
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

        entities.insert(EntityId::from_raw(0));
        assert_eq!(entities.get(EntityId::from_raw(0)), Some(ServerEntity(0)));
        assert_eq!(entities.get(ServerEntity(0)), Some(EntityId::from_raw(0)));
        assert_eq!(entities.len(), 1);

        entities.insert(EntityId::from_raw(1));
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
