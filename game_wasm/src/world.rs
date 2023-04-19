use core::mem::MaybeUninit;

use crate::raw::record::RecordReference;
use crate::raw::{Ptr, PtrMut, Usize};
use crate::Error;

use crate::raw::world::{self as raw};

#[derive(Clone)]
pub struct Entity(raw::Entity);

impl Entity {
    pub fn get(id: u64) -> Result<Self, Error> {
        let mut entity = MaybeUninit::<raw::Entity>::uninit();
        let ptr = entity.as_mut_ptr() as Usize;

        let res = unsafe { raw::world_entity_get(id, PtrMut::from_raw(ptr)) };

        if res == 0 {
            Ok(Self(unsafe { entity.assume_init_read() }))
        } else {
            Err(Error)
        }
    }

    pub fn spawn(&mut self) -> Result<(), Error> {
        let mut id = self.0.id;
        let out_ptr = &mut id as *mut u64 as Usize;

        let ptr = &self.0 as *const raw::Entity as Usize;

        let res = unsafe { raw::world_entity_spawn(Ptr::from_raw(ptr), PtrMut::from_raw(out_ptr)) };

        if res == 0 {
            self.0.id = id;

            Ok(())
        } else {
            Err(Error)
        }
    }

    pub fn despawn(&self) -> Result<(), Error> {
        let res = unsafe { raw::world_entity_despawn(self.0.id) };

        if res == 0 {
            Ok(())
        } else {
            Err(Error)
        }
    }

    pub fn components(&self) -> EntityComponents {
        EntityComponents { entity: self.0.id }
    }
}

#[derive(Clone, Debug)]
pub struct EntityComponents {
    entity: u64,
}

impl EntityComponents {
    pub fn get(&self, id: RecordReference) -> Result<Component, Error> {
        let mut component = MaybeUninit::<raw::Component>::uninit();
        let ptr = component.as_mut_ptr() as Usize;

        let res = unsafe {
            raw::world_entity_component_get(
                self.entity,
                Ptr::from_raw(&id as *const _ as Usize),
                PtrMut::from_raw(ptr),
            )
        };

        if res == 0 {
            let component = unsafe { component.assume_init_read() };

            Ok(match component {
                raw::Component::I32(x) => Component::I32(x),
                raw::Component::I64(x) => Component::I64(x),
            })
        } else {
            Err(Error)
        }
    }

    pub fn insert(&self, id: RecordReference, component: Component) -> Result<(), Error> {
        let c = match component {
            Component::I32(x) => raw::Component::I32(x),
            Component::I64(x) => raw::Component::I64(x),
        };
        let ptr = &c as *const raw::Component as Usize;

        let res = unsafe {
            raw::world_entity_component_insert(
                self.entity,
                Ptr::from_raw(&id as *const _ as Usize),
                Ptr::from_raw(ptr),
            )
        };

        if res == 0 {
            Ok(())
        } else {
            Err(Error)
        }
    }

    pub fn remove(&self, id: RecordReference) -> Result<(), Error> {
        let res = unsafe { raw::world_entity_component_remove(self.entity, id) };

        if res == 0 {
            Ok(())
        } else {
            Err(Error)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    I32(i32),
    I64(i64),
}
