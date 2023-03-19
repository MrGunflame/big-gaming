use std::borrow::Borrow;
use std::sync::Arc;
use std::time::Instant;

use ahash::HashMap;
use bevy::prelude::Resource;
use game_common::entity::EntityId;
use game_net::conn::{ConnectionHandle, ConnectionId};
use game_net::snapshot::{Command, EntityChange};
use parking_lot::RwLock;

use crate::net::state::ConnectionState;

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

    pub fn host<T>(&self, id: T) -> Option<EntityId>
    where
        T: Borrow<ConnectionId>,
    {
        let inner = self.connections.read();
        let data = inner.get(id.borrow())?.clone();
        let l = data.host.read();
        *l
    }

    pub fn get_mut<T>(&self, id: T) -> Option<ConnectionMut>
    where
        T: Borrow<ConnectionId>,
    {
        let mut inner = self.connections.write();

        match inner.get_mut(id.borrow()) {
            Some(data) => Some(ConnectionMut {
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

    pub fn remove<T>(&self, id: T)
    where
        T: Borrow<ConnectionId>,
    {
        let mut inner = self.connections.write();
        inner.remove(id.borrow());
    }
}

#[derive(Debug)]
pub struct ConnectionData {
    pub snapshot: RwLock<Vec<EntityChange>>,
    pub handle: ConnectionHandle,
    pub host: RwLock<Option<EntityId>>,
    pub state: RwLock<ConnectionState>,
}

impl ConnectionData {
    pub fn new(handle: ConnectionHandle) -> Self {
        Self {
            snapshot: RwLock::new(vec![]),
            handle,
            host: RwLock::new(None),
            state: RwLock::new(ConnectionState {
                full_update: true,
                cells: vec![],
                id: None,
                head: 0,
            }),
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
    id: ConnectionId,
}

impl ConnectionMut {
    pub fn set_delta(&self, delta: Vec<EntityChange>) {
        if delta.len() > 0 {
            // tracing::info!("write {} deltas to peer {:?}", delta.len(), self.id);
        }

        *self.data.snapshot.write() = delta;
    }
}

impl Drop for ConnectionMut {
    fn drop(&mut self) {
        // let mut prev = self.data.snapshot.write();
        // let delta = prev.delta(&self.snapshot);

        // *prev = self.snapshot.clone();

        // // Drop the lock as early as possible.
        // drop(prev);

        let delta = self.data.snapshot.read();

        for change in &*delta {
            let cmd = match change {
                EntityChange::Create { id, data } => Command::EntityCreate {
                    id: *id,
                    translation: data.transform.translation,
                    rotation: data.transform.rotation,
                    data: data.data.clone(),
                },
                EntityChange::Translate { id, translation } => Command::EntityTranslate {
                    id: *id,
                    translation: *translation,
                },
                EntityChange::Rotate { id, rotation } => Command::EntityRotate {
                    id: *id,
                    rotation: *rotation,
                },
                EntityChange::Destroy { id } => Command::EntityDestroy { id: *id },
                _ => unimplemented!(),
            };

            self.data.handle.send_cmd(cmd);
        }
    }
}
