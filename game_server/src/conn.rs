use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use ahash::HashMap;
use bevy::prelude::{Entity, Resource};
use game_common::net::ServerEntity;
use game_common::world::entity::Entity as EntityBody;
use game_net::conn::{ConnectionHandle, ConnectionId};
use game_net::proto::EntityKind;
use game_net::snapshot::{Command, EntityChange, Snapshot};
use parking_lot::RwLock;

/// List of connections
// FIXME: Maybe merge with ConnectionPool.
#[derive(Clone, Debug, Default, Resource)]
pub struct Connections {
    connections: Arc<RwLock<HashMap<ConnectionId, Arc<ConnectionData>>>>,
}

impl Connections {
    pub fn insert(&self, handle: ConnectionHandle) {
        let mut inner = self.connections.write();

        inner.insert(handle.id, Arc::new(ConnectionData::new(handle)));
    }

    pub fn set_host<T>(&self, id: T, host: Entity)
    where
        T: Borrow<ConnectionId>,
    {
        let mut inner = self.connections.write();

        let data = inner.get_mut(id.borrow()).unwrap();

        *data.host.write() = Some(host);
        data.handle.send_cmd(Command::SpawnHost { id: host });
    }

    pub fn get_mut<T>(&self, id: T) -> Option<ConnectionMut>
    where
        T: Borrow<ConnectionId>,
    {
        let mut inner = self.connections.write();

        match inner.get_mut(id.borrow()) {
            Some(data) => Some(ConnectionMut {
                snapshot: data.snapshot.read().clone(),
                data: data.clone(),
                id: *id.borrow(),
            }),
            None => None,
        }
    }

    pub fn iter_mut(&self) -> IterMut<'_> {
        let ids: Vec<_> = self.connections.read().keys().copied().collect();

        IterMut {
            inner: self,
            ids: ids.into_iter(),
        }
    }
}

#[derive(Debug)]
pub struct ConnectionData {
    pub snapshot: RwLock<Snapshot>,
    pub handle: ConnectionHandle,
    pub host: RwLock<Option<Entity>>,
}

impl ConnectionData {
    pub fn new(handle: ConnectionHandle) -> Self {
        Self {
            snapshot: RwLock::new(Snapshot::new()),
            handle,
            host: RwLock::new(None),
        }
    }
}

pub struct IterMut<'a> {
    inner: &'a Connections,
    ids: std::vec::IntoIter<ConnectionId>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = ConnectionMut;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.ids.next()?;
        self.inner.get_mut(id)
    }
}

pub struct ConnectionMut {
    pub data: Arc<ConnectionData>,
    /// The *new* snapshot (cloned from the one in data).
    snapshot: Snapshot,
    id: ConnectionId,
}

impl Deref for ConnectionMut {
    type Target = Snapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl DerefMut for ConnectionMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.snapshot
    }
}

impl Drop for ConnectionMut {
    fn drop(&mut self) {
        let mut prev = self.data.snapshot.write();
        let delta = prev.delta(&self.snapshot);

        *prev = self.snapshot.clone();

        // Drop the lock as early as possible.
        drop(prev);

        for change in delta {
            let cmd = match change {
                EntityChange::Create { id, content } => Command::EntityCreate {
                    id: ServerEntity(0),
                    kind: match &content {
                        EntityBody::Object(obj) => EntityKind::Object(obj.id),
                        EntityBody::Actor(act) => EntityKind::Actor(()),
                        _ => unimplemented!(),
                    },
                    translation: match &content {
                        EntityBody::Object(obj) => obj.transform.translation,
                        EntityBody::Actor(act) => act.transform.translation,
                        _ => unimplemented!(),
                    },
                    rotation: match &content {
                        EntityBody::Object(obj) => obj.transform.rotation,
                        EntityBody::Actor(act) => act.transform.rotation,
                        _ => unimplemented!(),
                    },
                },
                EntityChange::Update { id, content } => unimplemented!(),
                EntityChange::Destroy(id) => Command::EntityDestroy { id },
            };

            self.data.handle.send_cmd(cmd);
        }
    }
}
