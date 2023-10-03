use std::borrow::Borrow;
use std::iter::FusedIterator;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use ahash::HashMap;
use game_net::conn::ConnectionHandle;
use game_net::message::MessageId;
use parking_lot::{Mutex, RwLock};

use crate::net::state::ConnectionState;

static CONNECTION_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(u32);

impl Default for ConnectionId {
    fn default() -> Self {
        let id = CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(id, u32::MAX);
        Self(id)
    }
}

/// List of connections
// FIXME: Maybe merge with ConnectionPool.
#[derive(Clone, Debug, Default)]
pub struct Connections {
    connections: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
}

impl Connections {
    pub fn insert(&self, handle: Arc<ConnectionHandle>) {
        let mut inner = self.connections.write();
        let id = ConnectionId::default();

        inner.insert(
            id,
            Connection {
                inner: Arc::new(ConnectionInner {
                    id,
                    state: RwLock::new(ConnectionState::new()),
                    handle,
                    messages_in_frame: Mutex::new(vec![]),
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

    pub fn state(&self) -> &RwLock<ConnectionState> {
        &self.inner.state
    }

    pub fn push_message_in_frame(&self, id: MessageId) {
        self.inner.messages_in_frame.lock().push(id);
    }

    pub fn take_messages_in_frame(&self) -> Vec<MessageId> {
        std::mem::take(&mut *self.inner.messages_in_frame.lock())
    }
}

#[derive(Debug)]
struct ConnectionInner {
    id: ConnectionId,
    handle: Arc<ConnectionHandle>,
    state: RwLock<ConnectionState>,
    messages_in_frame: Mutex<Vec<MessageId>>,
}
