//!
//! FIXME: We should find a better name this system to prevent mixing it with bevy ECS components.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use bevy_ecs::system::Resource;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

static COMPONENTS: RwLock<Option<ComponentsInner>> = RwLock::new(None);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComponentId(u32);

#[derive(Clone, Debug, Resource)]
pub struct Components(&'static RwLock<Option<ComponentsInner>>);

#[derive(Debug, Default)]
pub struct ComponentsInner {
    next_id: u32,
    components: HashMap<ComponentId, ComponentInfo>,
    type_id: HashMap<TypeId, ComponentInfo>,
}

impl Components {
    pub fn new() -> Self {
        Self(&COMPONENTS)
    }

    pub fn get<T: Any>(&self) -> Option<ComponentInfo> {
        let inner = self.read();
        inner.type_id.get(&TypeId::of::<T>()).cloned()
    }

    pub fn get_by_id(&self, id: ComponentId) -> Option<ComponentInfo> {
        let inner = self.read();
        inner.components.get(&id).cloned()
    }

    pub fn insert(&mut self, info: ComponentInfo) -> ComponentId {
        let mut inner = self.write();

        let id = ComponentId(inner.next_id);
        inner.next_id += 1;

        if inner.next_id >= u32::MAX >> 1 {
            panic!("exceeded maximum ComponentId");
        }

        inner.components.insert(id, info.clone());

        if let Some(type_id) = info.type_id {
            inner.type_id.insert(type_id, info);
        }

        id
    }

    fn read(&self) -> ReadGuard<'_> {
        ReadGuard(self.0.read())
    }

    fn write(&self) -> WriteGuard<'_> {
        WriteGuard(self.0.write())
    }
}

struct ReadGuard<'a>(RwLockReadGuard<'a, Option<ComponentsInner>>);

impl<'a> Deref for ReadGuard<'a> {
    type Target = ComponentsInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

struct WriteGuard<'a>(RwLockWriteGuard<'a, Option<ComponentsInner>>);

impl<'a> Deref for WriteGuard<'a> {
    type Target = ComponentsInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl<'a> DerefMut for WriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.0.is_none() {
            *self.0 = Some(ComponentsInner::default());
        }

        self.0.as_mut().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub type_id: Option<TypeId>,
    pub encode: Option<EncodeFn>,
    pub decode: Option<DecodeFn>,
    pub drop: Option<DropFn>,
}

pub type EncodeFn = unsafe fn(*mut (), buf: *mut u8);
pub type DecodeFn = fn(ptr: *const u8, len: usize) -> *mut ();
pub type DropFn = unsafe fn(this: *mut ());

#[derive(Debug)]
pub struct DynamicComponent {
    id: ComponentId,
    encode: EncodeFn,
}

impl DynamicComponent {
    pub fn encode(&self) {}
}
