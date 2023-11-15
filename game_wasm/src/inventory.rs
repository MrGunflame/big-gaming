use core::iter::FusedIterator;
use core::mem::MaybeUninit;
use core::ops::Deref;

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};

use crate::component::{Component, Components};
use crate::entity::EntityId;
use crate::raw::inventory::{
    inventory_clear, inventory_component_get, inventory_component_insert, inventory_component_len,
    inventory_equip, inventory_get, inventory_insert, inventory_len, inventory_list,
    inventory_remove, inventory_unequip, Item as RawItem, ItemStack as RawItemStack,
};

use crate::raw::{Ptr, PtrMut, Usize};
use crate::record::{Record, RecordKind};
use crate::world::RecordReference;

/// A unique identifier to in [`ItemStack`] in an [`Inventory`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct InventoryId(u64);

impl InventoryId {
    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self(bits)
    }

    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

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

    /// Returns the number of [`ItemStack`]s contained in this `Inventory`.
    pub fn len(&self) -> Result<u32, InventoryError> {
        let mut len = MaybeUninit::uninit();

        let res = unsafe {
            inventory_len(
                self.entity.into_raw(),
                PtrMut::from_raw(len.as_mut_ptr() as Usize),
            )
        };
        match res {
            0 => Ok(unsafe { len.assume_init() }),
            _ => Err(InventoryError),
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

    pub fn keys(&self) -> Result<Keys, InventoryError> {
        let len = self.len()?;
        let mut keys = Vec::with_capacity(len.try_into().unwrap());

        let res = unsafe {
            inventory_list(
                self.entity.into_raw(),
                PtrMut::from_raw(keys.as_mut_ptr() as Usize),
                len,
            )
        };
        match res {
            0 => {
                unsafe { keys.set_len(len.try_into().unwrap()) };
                Ok(Keys {
                    inner: keys.into_iter(),
                })
            }
            _ => Err(InventoryError),
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

/// An `Iterator` over all the [`InventoryId`]s in an [`Inventory`].
#[derive(Clone, Debug)]
pub struct Keys {
    inner: alloc::vec::IntoIter<InventoryId>,
}

impl Iterator for Keys {
    type Item = InventoryId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Keys {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl FusedIterator for Keys {}

#[derive(Clone, Debug)]
pub struct ItemStackBuilder {
    id: RecordReference,
    components: Components,
    quantity: u32,
}

impl ItemStackBuilder {
    pub fn from_record(id: RecordReference) -> Self {
        let record = Record::get(id);
        assert_eq!(record.kind, RecordKind::Item);

        Self {
            id,
            components: record.components,
            quantity: 1,
        }
    }

    pub fn quantity(mut self, quantity: u32) -> Self {
        self.quantity = quantity;
        self
    }

    pub fn insert(&self, inventory: &mut Inventory) -> InventoryId {
        let mut slot_id = MaybeUninit::uninit();

        let stack = RawItemStack {
            item: RawItem { id: self.id },
            quantity: self.quantity,
        };

        let res = unsafe {
            inventory_insert(
                inventory.entity.into_raw(),
                Ptr::from_ptr(&stack),
                PtrMut::from_ptr(slot_id.as_mut_ptr()),
            )
        };
        assert!(res == 0);

        let slot_id = unsafe { slot_id.assume_init() };

        for (id, component) in &self.components {
            let res = unsafe {
                inventory_component_insert(
                    inventory.entity.into_raw(),
                    slot_id,
                    Ptr::from_ptr(&id),
                    Ptr::from_ptr(component.as_ptr()),
                    component.len() as u32,
                )
            };
            assert!(res == 0);
        }

        InventoryId(slot_id)
    }
}
