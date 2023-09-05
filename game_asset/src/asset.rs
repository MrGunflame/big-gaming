use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Arc;

use parking_lot::Mutex;
use slotmap::{DefaultKey, SlotMap};

pub trait Asset: Send + Sync + 'static {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HandleId(DefaultKey);

#[derive(Debug)]
pub struct Handle<T>
where
    T: Asset,
{
    id: HandleId,
    events: Arc<Mutex<VecDeque<Event>>>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T>
where
    T: Asset,
{
    pub fn id(&self) -> HandleId {
        self.id
    }
}

impl<T> Clone for Handle<T>
where
    T: Asset,
{
    fn clone(&self) -> Self {
        let mut events = self.events.lock();
        events.push_back(Event::Clone(self.id));

        Self {
            id: self.id,
            events: self.events.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Handle<T>
where
    T: Asset,
{
    fn drop(&mut self) {
        let mut events = self.events.lock();
        events.push_back(Event::Drop(self.id));
    }
}

#[derive(Clone, Debug, Default)]
pub struct Assets<T>
where
    T: Asset,
{
    assets: SlotMap<DefaultKey, Entry<T>>,
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl<T> Assets<T>
where
    T: Asset,
{
    pub fn new() -> Self {
        Self {
            assets: SlotMap::new(),
            events: Default::default(),
        }
    }

    pub fn insert(&mut self, asset: T) -> Handle<T> {
        let id = self.assets.insert(Entry {
            asset,
            ref_count: 1,
        });

        Handle {
            id: HandleId(id),
            events: self.events.clone(),
            _marker: PhantomData,
        }
    }

    pub fn remove(&mut self, id: HandleId) -> Option<T> {
        self.assets.remove(id.0).map(|e| e.asset)
    }

    pub fn get(&self, id: HandleId) -> Option<&T> {
        self.assets.get(id.0).map(|e| &e.asset)
    }

    pub fn get_mut(&mut self, id: HandleId) -> Option<&mut T> {
        self.assets.get_mut(id.0).map(|e| &mut e.asset)
    }

    pub fn flush_events(&mut self) {
        let mut events = self.events.lock();

        while let Some(event) = events.pop_front() {
            match event {
                Event::Clone(id) => {
                    let asset = self.assets.get_mut(id.0).unwrap();
                    asset.ref_count += 1;
                }
                Event::Drop(id) => {
                    let asset = self.assets.get_mut(id.0).unwrap();
                    asset.ref_count -= 1;

                    if asset.ref_count == 0 {
                        self.assets.remove(id.0);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Entry<T: Asset> {
    asset: T,
    ref_count: usize,
}

#[derive(Copy, Clone, Debug)]
enum Event {
    Clone(HandleId),
    Drop(HandleId),
}
