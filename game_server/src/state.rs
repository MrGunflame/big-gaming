use std::borrow::Borrow;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;

use ahash::AHashMap;
use game_common::world::control_frame::ControlFrame;
use game_net::proto::Packet;
use parking_lot::{Mutex, RwLock};
use tokio::sync::mpsc;

use crate::config::Config;
use crate::conn::Connections;

pub type ConnectionKey = SocketAddr;

#[derive(Clone, Debug)]
pub struct State(Arc<StateInner>);

impl State {
    pub fn new(config: Config) -> Self {
        State(Arc::new(StateInner {
            config,
            pool: ConnectionPool::new(),
            conns: Connections::default(),
            control_frame: Mutex::default(),
        }))
    }
}

impl Deref for State {
    type Target = StateInner;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct StateInner {
    pub config: Config,
    pub pool: ConnectionPool,

    pub conns: Connections,
    // TODO: This can probably be AtomicU32, but needs to be consitent.
    pub control_frame: Mutex<ControlFrame>,
}

#[derive(Debug)]
pub struct ConnectionPool {
    inner: RwLock<AHashMap<ConnectionKey, mpsc::Sender<Packet>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            inner: RwLock::default(),
        }
    }

    pub fn insert(&self, key: ConnectionKey, handle: mpsc::Sender<Packet>) {
        let mut inner = self.inner.write();
        inner.insert(key, handle);
    }

    pub fn remove<K>(&self, key: K)
    where
        K: Borrow<ConnectionKey>,
    {
        let mut inner = self.inner.write();
        inner.remove(key.borrow());
    }

    pub fn get<K>(&self, key: K) -> Option<mpsc::Sender<Packet>>
    where
        K: Borrow<ConnectionKey>,
    {
        let inner = self.inner.read();
        inner.get(key.borrow()).cloned()
    }

    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
