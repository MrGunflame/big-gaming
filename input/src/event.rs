use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::input::ButtonState;
use bevy::prelude::{KeyCode, ResMut};
use bevy_ecs::system::{Local, Resource, SystemParam};
use bevy_ecs::world::{FromWorld, World};

pub trait Event: Clone + Send + Sync + 'static {}

#[derive(Debug, Resource)]
pub struct Events<E>
where
    E: Event,
{
    /// The id of the first event in the collection.
    head: EventId<E>,
    /// The readable events in the collection.
    // id => (reads, event)
    events: HashMap<EventId<E>, (usize, E)>,
    next_id: AtomicU32,
    num_rx: usize,
}

impl<E> Events<E>
where
    E: Event,
{
    fn push(&mut self, event: E) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        self.events.insert(EventId::new(id), (0, event));
    }

    fn get(&mut self, id: EventId<E>) -> Option<E> {
        let (reads, event) = self.events.get_mut(&id)?;
        *reads += 1;

        // Event is now observed by all readers.
        if *reads >= self.num_rx {
            if self.head == id {
                self.head.id += 1;
            }

            return self.events.remove(&id).map(|(_, e)| e);
        }

        Some(event.clone())
    }

    fn remove(&mut self, id: EventId<E>) -> Option<E> {
        let (_, event) = self.events.remove(&id)?;

        if self.head == id {
            self.head.id += 1;
        }

        Some(event)
    }
}

#[derive(Debug, SystemParam)]
pub struct EventWriter<'w, 's, E>
where
    E: Event,
{
    inner: ResMut<'w, Events<E>>,
    #[system_param(ignore)]
    _marker: PhantomData<&'s ()>,
}

impl<'w, 's, E> EventWriter<'w, 's, E>
where
    E: Event,
{
    pub fn send(&mut self, event: E) {
        self.inner.push(event);
    }
}

impl<'w, 's, E> Extend<E> for EventWriter<'w, 's, E>
where
    E: Event,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = E>,
    {
        for event in iter {
            self.send(event);
        }
    }
}

#[derive(Debug, SystemParam)]
pub struct EventReader<'w, 's, E>
where
    E: Event,
{
    /// The next expected [`EventId`] of this `EventReader`. This is, in other words, the same as
    /// `last + 1`.
    next: Local<'s, EventId<E>>,
    inner: ResMut<'w, Events<E>>,
}

impl<'w, 's, E> EventReader<'w, 's, E>
where
    E: Event,
{
    pub fn iter(&mut self) -> Iter<'_, 'w, 's, E> {
        Iter { inner: self }
    }
}

pub struct Iter<'a, 'w, 's, E>
where
    E: Event,
{
    inner: &'a mut EventReader<'w, 's, E>,
}

impl<'a, 'w, 's, E> Iterator for Iter<'a, 'w, 's, E>
where
    E: Event,
{
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

pub struct KeyInput {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ButtonState,
}

pub struct EventId<E>
where
    E: Event,
{
    id: u32,
    _marker: PhantomData<E>,
}

impl<E> Clone for EventId<E>
where
    E: Event,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.id)
    }
}

impl<E> Copy for EventId<E> where E: Event {}

impl<E> PartialEq for EventId<E>
where
    E: Event,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E> Eq for EventId<E> where E: Event {}

impl<E> EventId<E>
where
    E: Event,
{
    #[inline]
    fn new(id: u32) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }
}

impl<E> Hash for EventId<E>
where
    E: Event,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<E> FromWorld for EventId<E>
where
    E: Event,
{
    #[inline]
    fn from_world(world: &mut World) -> Self {
        let events = world.resource::<Events<E>>();
        events.head
    }
}

impl<E> Debug for EventId<E>
where
    E: Event,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventId")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
