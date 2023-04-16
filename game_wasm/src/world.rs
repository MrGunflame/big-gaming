use core::mem::MaybeUninit;

use crate::raw::{Ptr, PtrMut, Usize};
use crate::Error;

use crate::raw::world as raw;

pub struct Entity(raw::Entity);

impl Entity {
    pub fn get(id: u64) -> Result<Self, Error> {
        let mut entity = MaybeUninit::<Entity>::uninit();
        let ptr = entity.as_mut_ptr() as Usize;

        let res = unsafe { raw::world_entity_get(id, PtrMut::from_raw(ptr)) };

        if res == 0 {
            Ok(unsafe { entity.assume_init_read() })
        } else {
            Err(Error)
        }
    }

    pub fn spawn(&self) -> Result<(), Error> {
        let ptr = &self.0 as *const raw::Entity as Usize;

        let res = unsafe { raw::world_entity_spawn(Ptr::from_raw(ptr)) };

        if res == 0 {
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
}
