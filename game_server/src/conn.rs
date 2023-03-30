use std::borrow::Borrow;
use std::iter::FusedIterator;
use std::sync::Arc;

use ahash::HashMap;
use bevy::prelude::Resource;
use game_common::entity::EntityId;
use game_common::world::snapshot::EntityChange;
use game_net::conn::{ConnectionHandle, ConnectionId};
use game_net::snapshot::Command;
use parking_lot::RwLock;

use crate::net::state::ConnectionState;

/// List of connections
// FIXME: Maybe merge with ConnectionPool.
#[derive(Clone, Debug, Default, Resource)]
pub struct Connections {
    connections: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
}

impl Connections {
    pub fn insert(&self, handle: ConnectionHandle) {
        let mut inner = self.connections.write();

        inner.insert(
            handle.id,
            Connection {
                inner: Arc::new(ConnectionInner {
                    id: handle.id,
                    state: RwLock::new(ConnectionState {
                        full_update: true,
                        cells: vec![],
                        id: None,
                        head: 0,
                    }),
                    handle,
                }),
            },
        );
    }

    pub fn get<T>(&self, id: T) -> Option<Connection>
    where
        T: Borrow<ConnectionId>,
    {
        let inner = self.connections.read();
        inner.get(id.borrow()).cloned()
    }

    pub fn iter(&self) -> Iter<'_> {
        let ids: Vec<_> = self.connections.read().keys().copied().collect();

        Iter {
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

impl<'a> IntoIterator for &'a Connections {
    type Item = Connection;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub struct Iter<'a> {
    inner: &'a Connections,
    ids: std::vec::IntoIter<ConnectionId>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Connection;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let id = self.ids.next()?;

            // Note: It is possible that the connection was already removed
            // by another thread while iterting.
            if let Some(conn) = self.inner.get(id) {
                return Some(conn);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.ids.len()))
    }
}

impl<'a> FusedIterator for Iter<'a> {}

#[derive(Clone, Debug)]
pub struct Connection {
    inner: Arc<ConnectionInner>,
}

impl Connection {
    pub fn id(&self) -> ConnectionId {
        self.inner.id
    }

    pub fn handle(&self) -> &ConnectionHandle {
        &self.inner.handle
    }

    pub fn push<T>(&self, deltas: T)
    where
        T: IntoDeltas,
    {
        for delta in deltas.into_deltas() {
            let cmd = match delta {
                EntityChange::Create { entity } => Command::EntityCreate {
                    id: entity.id,
                    translation: entity.transform.translation,
                    rotation: entity.transform.rotation,
                    data: entity.body,
                },
                EntityChange::Destroy { id } => Command::EntityDestroy { id },
                EntityChange::Translate {
                    id,
                    translation,
                    cell: _,
                } => Command::EntityTranslate { id, translation },
                EntityChange::Rotate { id, rotation } => Command::EntityRotate { id, rotation },
                _ => todo!(),
            };

            self.inner.handle.send_cmd(cmd);
        }
    }

    pub fn state(&self) -> &RwLock<ConnectionState> {
        &self.inner.state
    }

    pub fn host(&self) -> Option<EntityId> {
        self.state().read().id
    }

    pub fn set_host(&self, id: EntityId) {
        self.state().write().id = Some(id);
        self.handle().send_cmd(Command::SpawnHost { id });
    }
}

#[derive(Debug)]
struct ConnectionInner {
    id: ConnectionId,
    handle: ConnectionHandle,
    state: RwLock<ConnectionState>,
}

pub trait IntoDeltas {
    fn into_deltas(self) -> Vec<EntityChange>;
}

impl IntoDeltas for EntityChange {
    fn into_deltas(self) -> Vec<EntityChange> {
        vec![self]
    }
}

impl IntoDeltas for Vec<EntityChange> {
    fn into_deltas(self) -> Vec<EntityChange> {
        self
    }
}
