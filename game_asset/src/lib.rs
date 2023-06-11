//! Asset loader
//!

mod asset;
mod io;

use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::EventWriter;
use bevy_ecs::system::{ResMut, Resource};
use parking_lot::Mutex;
use slotmap::{DefaultKey, SlotMap};
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio::task::AbortHandle;

pub use crate::asset::{Asset, Assets, Handle, HandleId};

#[derive(Clone, Debug, Default)]
pub struct AssetPlugin {}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetServer::new());
        app.add_event::<AssetEvent>();
        app.add_system(flush_asset_server_events);
    }
}

pub trait AssetAppExt {
    fn add_asset<T: Asset>(&mut self);
}

#[derive(Debug, Resource)]
pub struct AssetServer {
    assets: SlotMap<DefaultKey, Entry>,
    events: Arc<Mutex<VecDeque<Event>>>,
    rt: Runtime,
    tasks: HashMap<DefaultKey, AbortHandle>,
}

impl AssetServer {
    pub fn new() -> Self {
        let rt = Runtime::new().unwrap();

        Self {
            assets: SlotMap::new(),
            events: Arc::default(),
            rt,
            tasks: HashMap::new(),
        }
    }

    pub fn load<P>(&mut self, path: P) -> AssetHandle
    where
        P: AsRef<Path>,
    {
        let id = self.assets.insert(Entry {
            data: None,
            ref_count: 1,
        });

        let path = path.as_ref().to_owned();
        let events = self.events.clone();
        let handle = self.rt.spawn(async move {
            let buf = io::load_file(path).await.unwrap();
            let mut events = events.lock();
            events.push_back(Event::Create(AssetId(id), buf));
        });

        self.tasks.insert(id, handle.abort_handle());

        AssetHandle {
            id: AssetId(id),
            events: self.events.clone(),
        }
    }

    pub fn get(&mut self, id: AssetId) -> Result<&[u8], Error> {
        match self.assets.get(id.0) {
            Some(asset) => match &asset.data {
                Some(buf) => Ok(buf),
                None => Err(Error::Loading),
            },
            None => Err(Error::InvalidId),
        }
    }

    pub fn remove(&mut self, id: AssetId) {
        self.assets.remove(id.0);

        if let Some(handle) = self.tasks.remove(&id.0) {
            handle.abort();
        }
    }
}

pub enum AssetServerEvent {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetId(DefaultKey);

pub struct AssetHandle {
    id: AssetId,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl Clone for AssetHandle {
    fn clone(&self) -> Self {
        let mut events = self.events.lock();
        events.push_back(Event::Clone(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
        }
    }
}

impl Drop for AssetHandle {
    fn drop(&mut self) {
        let mut events = self.events.lock();
        events.push_back(Event::Drop(self.id));
    }
}

impl PartialEq for AssetHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AssetHandle {}

#[derive(Clone, Debug)]
struct Entry {
    data: Option<Vec<u8>>,
    ref_count: usize,
}

#[derive(Debug)]
enum Event {
    Clone(AssetId),
    Drop(AssetId),
    Create(AssetId, Vec<u8>),
}

fn flush_asset_server_events(
    mut server: ResMut<AssetServer>,
    mut asset_events: EventWriter<AssetEvent>,
) {
    let server = &mut *server;

    let mut events = server.events.lock();

    while let Some(event) = events.pop_front() {
        match event {
            Event::Clone(id) => {
                let asset = server.assets.get_mut(id.0).unwrap();
                asset.ref_count += 1;
            }
            Event::Drop(id) => {
                let asset = server.assets.get_mut(id.0).unwrap();
                asset.ref_count -= 1;

                if asset.ref_count == 0 {
                    server.assets.remove(id.0);

                    asset_events.send(AssetEvent::Destroyed { id });
                }
            }
            Event::Create(id, buf) => {
                server.tasks.remove(&id.0);

                // If `None` all handles are already removed.
                if let Some(asset) = server.assets.get_mut(id.0) {
                    asset.data = Some(buf);

                    asset_events.send(AssetEvent::Created { id });
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AssetEvent {
    Created { id: AssetId },
    Destroyed { id: AssetId },
}

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("invalid asset id")]
    InvalidId,
    #[error("asset is still loading")]
    Loading,
}

pub trait LoadAsset: Sized {
    type Error: Display;

    fn load(bytes: &[u8]) -> Result<Self, Self::Error>;
}
