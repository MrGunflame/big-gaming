use std::borrow::Borrow;
use std::collections::hash_map::{Values, ValuesMut};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};

use ahash::HashMap;
use bevy::prelude::{Entity, Resource};
use game_net::conn::{ConnectionHandle, ConnectionId};
use game_net::snapshot::{Command, EntityChange, Snapshot};

/// List of connections
#[derive(Resource)]
pub struct Connections {
    snapshots: HashMap<ConnectionId, Snapshot>,
    handles: HashMap<ConnectionId, ConnectionHandle>,
    hosts: HashMap<ConnectionId, Option<Entity>>,
}

impl Connections {
    pub fn insert(&mut self, handle: ConnectionHandle) -> ConnectionId {
        let id = ConnectionId::new();
        self.snapshots.insert(id, Snapshot::new());
        self.handles.insert(id, handle);
        self.hosts.insert(id, None);
        id
    }

    pub fn set_host<T>(&mut self, id: T, host: Entity)
    where
        T: Borrow<ConnectionId>,
    {
        *self.hosts.get_mut(id.borrow()).unwrap() = Some(host);
        self.handles
            .get(id.borrow())
            .unwrap()
            .send_cmd(Command::SpawnHost { id: host });
    }

    pub fn get_mut<T>(&mut self, id: T) -> Option<ConnectionMut<'_>>
    where
        T: Borrow<ConnectionId>,
    {
        match self.snapshots.get_mut(id.borrow()) {
            Some(snap) => Some(ConnectionMut {
                handle: self.handles.get(id.borrow()).unwrap(),
                prev: snap.clone(),
                snapshot: snap,
                id: *id.borrow(),
            }),
            None => None,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            first: false,
            snapshots: self.snapshots.values_mut(),
            handles: self.handles.values_mut(),
        }
    }
}

pub struct IterMut<'a> {
    first: bool,
    snapshots: ValuesMut<'a, ConnectionId, Snapshot>,
    handles: ValuesMut<'a, ConnectionId, ConnectionHandle>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = &'a mut Snapshot;

    fn next(&mut self) -> Option<Self::Item> {
        let snapshot = self.snapshots.next();

        if self.first {
            // Calculate delta
        }

        if snapshot.is_some() {
            self.first = true;
        }

        snapshot
    }
}

pub struct ConnectionMut<'a> {
    handle: &'a ConnectionHandle,
    snapshot: &'a mut Snapshot,
    prev: Snapshot,
    id: ConnectionId,
}

impl<'a> Deref for ConnectionMut<'a> {
    type Target = Snapshot;

    fn deref(&self) -> &Self::Target {
        self.snapshot
    }
}

impl<'a> DerefMut for ConnectionMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.snapshot
    }
}

impl<'a> Drop for ConnectionMut<'a> {
    fn drop(&mut self) {
        let delta = self.prev.delta(&self.snapshot);

        for change in delta {
            let cmd = match change {
                EntityChange::Create { id, content } => unimplemented!(),
                EntityChange::Update { id, content } => unimplemented!(),
                EntityChange::Destroy(id) => Command::EntityDestroy { id },
            };

            self.handle.send_cmd(cmd);
        }
    }
}
