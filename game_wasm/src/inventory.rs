use core::mem::MaybeUninit;

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};

use crate::component::Component;
use crate::entity::EntityId;
use crate::raw::inventory::{
    inventory_component_get, inventory_component_insert, inventory_component_len, inventory_get,
    Item as RawItem,
};

use crate::raw::{Ptr, PtrMut, Usize};
use crate::world::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct InventoryId(pub u64);

pub struct Inventory {
    entity: EntityId,
}

impl Inventory {
    pub fn new(entity: EntityId) -> Self {
        Self { entity }
    }

    pub fn get(&self, id: InventoryId) -> Result<Item, InventoryError> {
        let mut item = MaybeUninit::<RawItem>::uninit();
        let ptr = item.as_mut_ptr() as Usize;

        let res = unsafe { inventory_get(self.entity.into_raw(), id.0, PtrMut::from_raw(ptr)) };

        if res == 0 {
            let item = unsafe { item.assume_init() };
            Ok(Item { id: item.id })
        } else {
            Err(InventoryError)
        }
    }

    pub fn component_get(
        &self,
        id: InventoryId,
        component_id: RecordReference,
    ) -> Result<Component, InventoryError> {
        let mut len: Usize = 0;
        let len_ptr = &mut len as *mut Usize as Usize;

        let res = unsafe {
            inventory_component_len(
                self.entity.into_raw(),
                id.0,
                Ptr::from_raw(&component_id as *const _ as Usize),
                PtrMut::from_raw(len_ptr),
            )
        };

        if res != 0 {
            return Err(InventoryError);
        }

        // No need to fetch any data if it is empty.
        if len == 0 {
            return Ok(Component::new(Vec::new()));
        }

        let mut bytes = Vec::with_capacity(len as usize);

        let res = unsafe {
            inventory_component_get(
                self.entity.into_raw(),
                id.0,
                Ptr::from_raw(&component_id as *const _ as Usize),
                PtrMut::from_raw(bytes.as_mut_ptr() as Usize),
                len,
            )
        };

        if res == 0 {
            unsafe {
                bytes.set_len(len as usize);
            }

            Ok(Component::new(bytes))
        } else {
            Err(InventoryError)
        }
    }

    pub fn component_insert(
        &self,
        id: InventoryId,
        component_id: RecordReference,
        component: &Component,
    ) -> Result<(), InventoryError> {
        let ptr = Ptr::from_raw(component.as_bytes().as_ptr() as Usize);
        let len = component.as_bytes().len() as Usize;

        let res = unsafe {
            inventory_component_insert(
                self.entity.into_raw(),
                id.0,
                Ptr::from_raw(&component_id as *const _ as Usize),
                ptr,
                len,
            )
        };

        if res == 0 {
            Ok(())
        } else {
            Err(InventoryError)
        }
    }
}

pub struct Item {
    pub id: RecordReference,
}

#[derive(Clone, Debug)]
pub struct InventoryError;
