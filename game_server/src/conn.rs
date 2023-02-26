use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use ahash::HashMap;
use bevy::prelude::Resource;
use game_common::entity::{EntityData, EntityId};
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

    pub fn set_host<T>(&self, id: T, host: EntityId)
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
    pub host: RwLock<Option<EntityId>>,
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
                EntityChange::Create { id, data } => {
                    dbg!(id);

                    Command::EntityCreate {
                        id,
                        kind: match data.data {
                            EntityData::Object { id } => EntityKind::Object(id),
                            EntityData::Actor {} => EntityKind::Actor(()),
                        },
                        translation: data.transform.translation,
                        rotation: data.transform.rotation,
                    }
                }
                EntityChange::Translate { id, translation } => {
                    Command::EntityTranslate { id, translation }
                }
                EntityChange::Rotate { id, rotation } => Command::EntityRotate { id, rotation },
                EntityChange::Destroy { id } => Command::EntityDestroy { id },
            };

            self.data.handle.send_cmd(cmd);
        }
    }
}
