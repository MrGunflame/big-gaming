use core::mem::MaybeUninit;
use core::ops::Deref;

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};

use crate::component::Component;
use crate::entity::EntityId;
use crate::raw::inventory::{
    inventory_clear, inventory_component_get, inventory_component_insert, inventory_component_len,
    inventory_equip, inventory_get, inventory_insert, inventory_remove, inventory_unequip,
    Item as RawItem, ItemStack as RawItemStack,
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

    pub fn get(&self, id: InventoryId) -> Result<ItemStackRef, InventoryError> {
        let mut stack = MaybeUninit::<RawItemStack>::uninit();
        let ptr = stack.as_mut_ptr() as Usize;

        let res = unsafe { inventory_get(self.entity.into_raw(), id.0, PtrMut::from_raw(ptr)) };

        if res == 0 {
            let stack = unsafe { stack.assume_init() };
            Ok(ItemStackRef {
                inner: ItemStack {
                    item: Item { id: stack.item.id },
                    quantity: stack.quantity,
                },
                slot_id: id,
                entity_id: self.entity,
            })
        } else {
            Err(InventoryError)
        }
    }

    pub fn insert<T>(&self, items: T) -> Result<InventoryId, InventoryError>
    where
        T: IntoItemStack,
    {
        self.insert_inner(items.into_item_stack())
    }

    fn insert_inner(&self, items: ItemStack) -> Result<InventoryId, InventoryError> {
        let raw_stack = RawItemStack {
            item: RawItem { id: items.item.id },
            quantity: items.quantity,
        };

        let mut slot_id = MaybeUninit::uninit();

        let res = unsafe {
            inventory_insert(
                self.entity.into_raw(),
                Ptr::from_raw(&raw_stack as *const _ as Usize),
                PtrMut::from_raw(slot_id.as_mut_ptr() as Usize),
            )
        };

        match res {
            0 => {
                let slot_id = unsafe { slot_id.assume_init() };
                Ok(slot_id)
            }
            _ => Err(InventoryError),
        }
    }

    pub fn remove(&self, slot_id: InventoryId, quantity: u64) -> Result<(), InventoryError> {
        let res = unsafe { inventory_remove(self.entity.into_raw(), slot_id.0, quantity) };
        match res {
            0 => Ok(()),
            _ => Err(InventoryError),
        }
    }

    pub fn clear(&mut self) -> Result<(), InventoryError> {
        match unsafe { inventory_clear(self.entity.into_raw()) } {
            0 => Ok(()),
            _ => Err(InventoryError),
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

#[derive(Copy, Clone, Debug)]
pub struct ItemStack {
    pub item: Item,
    pub quantity: u32,
}

#[derive(Debug)]
pub struct ItemStackRef {
    inner: ItemStack,
    entity_id: EntityId,
    slot_id: InventoryId,
}

impl ItemStackRef {
    pub fn equip(&mut self, equipped: bool) -> Result<(), InventoryError> {
        let res = if equipped {
            unsafe { inventory_equip(self.entity_id.into_raw(), self.slot_id.0) }
        } else {
            unsafe { inventory_unequip(self.entity_id.into_raw(), self.slot_id.0) }
        };

        match res {
            0 => Ok(()),
            _ => Err(InventoryError),
        }
    }
}

impl Deref for ItemStackRef {
    type Target = ItemStack;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AsRef<ItemStack> for ItemStackRef {
    #[inline]
    fn as_ref(&self) -> &ItemStack {
        &self.inner
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Item {
    pub id: RecordReference,
}

#[derive(Clone, Debug)]
pub struct InventoryError;

pub trait IntoItemStack: private::Sealed {}

mod private {
    use super::ItemStack;

    pub trait Sealed {
        fn into_item_stack(self) -> ItemStack;
    }
}

impl IntoItemStack for ItemStack {}
impl IntoItemStack for Item {}

impl private::Sealed for ItemStack {
    #[inline]
    fn into_item_stack(self) -> ItemStack {
        self
    }
}

impl private::Sealed for Item {
    #[inline]
    fn into_item_stack(self) -> ItemStack {
        ItemStack {
            item: self,
            quantity: 1,
        }
    }
}
