use core::iter::FusedIterator;
use core::mem::MaybeUninit;
use core::ops::Deref;

use alloc::vec::Vec;
use bytemuck::{Pod, Zeroable};

use crate::components::{Components, RawComponent};
use crate::encoding::{Decode, Encode};
use crate::entity::EntityId;
use crate::raw::inventory::{
    inventory_clear, inventory_component_get, inventory_component_insert, inventory_component_len,
    inventory_component_remove, inventory_equip, inventory_get, inventory_insert, inventory_len,
    inventory_list, inventory_remove, inventory_unequip, Item as RawItem,
    ItemStack as RawItemStack,
};
use crate::{unreachable_unchecked, Error, ErrorImpl};

use crate::raw::{RESULT_NO_COMPONENT, RESULT_NO_ENTITY, RESULT_NO_INVENTORY_SLOT, RESULT_OK};
use crate::record::{Record, RecordKind};
use crate::world::RecordReference;

/// A unique identifier to in [`ItemStack`] in an [`Inventory`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod, Encode, Decode)]
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

#[derive(Debug)]
pub struct Inventory {
    entity: EntityId,
}

impl Inventory {
    #[inline]
    pub fn new(entity: EntityId) -> Self {
        Self { entity }
    }

    pub fn get(&self, id: InventoryId) -> Result<ItemStackRef, Error> {
        let mut stack = MaybeUninit::<RawItemStack>::uninit();

        let res = unsafe { inventory_get(self.entity.into_raw(), id.0, stack.as_mut_ptr()) };

        match res {
            RESULT_OK => {
                let stack = unsafe { stack.assume_init() };
                Ok(ItemStackRef {
                    inner: ItemStack {
                        item: Item {
                            id: stack.item.id,
                            equipped: stack.item.equipped != 0,
                            hidden: stack.item.hdden != 0,
                        },
                        quantity: stack.quantity,
                    },
                    slot_id: id,
                    entity_id: self.entity,
                })
            }
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            RESULT_NO_INVENTORY_SLOT => Err(ErrorImpl::NoInventorySlot(id).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    /// Returns the number of [`ItemStack`]s contained in this `Inventory`.
    pub fn len(&self) -> Result<u32, Error> {
        let mut len = MaybeUninit::uninit();

        let res = unsafe { inventory_len(self.entity.into_raw(), len.as_mut_ptr()) };
        match res {
            RESULT_OK => Ok(unsafe { len.assume_init() }),
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn insert<T>(&self, items: T) -> Result<InventoryId, Error>
    where
        T: IntoItemStack,
    {
        self.insert_inner(items.into_item_stack())
    }

    fn insert_inner(&self, items: ItemStack) -> Result<InventoryId, Error> {
        let raw_stack = RawItemStack {
            item: RawItem {
                id: items.item.id,
                equipped: 0,
                hdden: 0,
                _pad0: 0,
            },
            quantity: items.quantity,
        };

        let mut slot_id = MaybeUninit::uninit();

        let res =
            unsafe { inventory_insert(self.entity.into_raw(), &raw_stack, slot_id.as_mut_ptr()) };

        match res {
            RESULT_OK => {
                let slot_id = unsafe { slot_id.assume_init() };
                Ok(InventoryId(slot_id))
            }
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn remove(&self, slot_id: InventoryId, quantity: u64) -> Result<(), Error> {
        let res = unsafe { inventory_remove(self.entity.into_raw(), slot_id.0, quantity) };
        match res {
            RESULT_OK => Ok(()),
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            RESULT_NO_INVENTORY_SLOT => Err(ErrorImpl::NoInventorySlot(slot_id).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        match unsafe { inventory_clear(self.entity.into_raw()) } {
            RESULT_OK => Ok(()),
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn component_get(
        &self,
        id: InventoryId,
        component_id: RecordReference,
    ) -> Result<RawComponent, Error> {
        let mut len: usize = 0;

        let res = unsafe {
            inventory_component_len(self.entity.into_raw(), id.0, &component_id, &mut len)
        };

        match res {
            RESULT_OK => (),
            RESULT_NO_ENTITY => return Err(ErrorImpl::NoEntity(self.entity).into_error()),
            RESULT_NO_COMPONENT => return Err(ErrorImpl::NoComponent(component_id).into_error()),
            RESULT_NO_INVENTORY_SLOT => return Err(ErrorImpl::NoInventorySlot(id).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }

        // No need to fetch any data if it is empty.
        if len == 0 {
            return Ok(RawComponent::new(Vec::new(), Vec::new()));
        }

        let mut bytes = Vec::with_capacity(len as usize);

        let res = unsafe {
            inventory_component_get(
                self.entity.into_raw(),
                id.0,
                &component_id,
                bytes.as_mut_ptr(),
                len,
            )
        };

        // The call to `inventory_component_get` should never fail since `inventory_component_len`
        // was successful and the VM guarantees that we have "exclusive" access to the entity.
        debug_assert!(res == RESULT_OK);
        unsafe {
            bytes.set_len(len as usize);
        }

        Ok(RawComponent::new(bytes, Vec::new()))
    }

    pub fn component_insert(
        &self,
        id: InventoryId,
        component_id: RecordReference,
        component: &RawComponent,
    ) -> Result<(), Error> {
        let res = unsafe {
            inventory_component_insert(
                self.entity.into_raw(),
                id.0,
                &component_id,
                component.as_bytes().as_ptr(),
                component.as_bytes().len(),
            )
        };

        match res {
            RESULT_OK => Ok(()),
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            RESULT_NO_INVENTORY_SLOT => Err(ErrorImpl::NoInventorySlot(id).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn component_remove(
        &self,
        id: InventoryId,
        component_id: RecordReference,
    ) -> Result<(), Error> {
        let res =
            unsafe { inventory_component_remove(self.entity.into_raw(), id.0, &component_id) };

        match res {
            RESULT_OK => Ok(()),
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            RESULT_NO_COMPONENT => Err(ErrorImpl::NoComponent(component_id).into_error()),
            RESULT_NO_INVENTORY_SLOT => Err(ErrorImpl::NoInventorySlot(id).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn keys(&self) -> Result<Keys, Error> {
        let len = self.len()?;
        let mut keys = Vec::with_capacity(len.try_into().unwrap());

        let res = unsafe { inventory_list(self.entity.into_raw(), keys.as_mut_ptr(), len) };
        match res {
            RESULT_OK => {
                unsafe { keys.set_len(len.try_into().unwrap()) };
                Ok(Keys {
                    inner: keys.into_iter(),
                })
            }
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn iter(&self) -> Result<Iter<'_>, Error> {
        Ok(Iter {
            keys: self.keys()?,
            inventory: self,
        })
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
    pub fn equip(&mut self, equipped: bool) -> Result<(), Error> {
        let res = if equipped {
            unsafe { inventory_equip(self.entity_id.into_raw(), self.slot_id.0) }
        } else {
            unsafe { inventory_unequip(self.entity_id.into_raw(), self.slot_id.0) }
        };

        match res {
            RESULT_OK => Ok(()),
            RESULT_NO_ENTITY => Err(ErrorImpl::NoEntity(self.entity_id).into_error()),
            RESULT_NO_INVENTORY_SLOT => Err(ErrorImpl::NoInventorySlot(self.slot_id).into_error()),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub fn components(&self) -> ItemComponents<'_> {
        ItemComponents { parent: self }
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

#[derive(Debug)]
pub struct ItemComponents<'a> {
    parent: &'a ItemStackRef,
}

impl<'a> ItemComponents<'a> {
    pub fn get(&self, id: RecordReference) -> Result<RawComponent, Error> {
        Inventory::new(self.parent.entity_id).component_get(self.parent.slot_id, id)
    }

    pub fn insert(&self, id: RecordReference, component: &RawComponent) -> Result<(), Error> {
        Inventory::new(self.parent.entity_id).component_insert(self.parent.slot_id, id, component)
    }

    pub fn remove(&self, id: RecordReference) -> Result<(), Error> {
        Inventory::new(self.parent.entity_id).component_remove(self.parent.slot_id, id)
    }

    pub fn entry(&self, id: RecordReference) -> ComponentEntry<'_> {
        match self.get(id) {
            Ok(component) => ComponentEntry::Occupied(OccupiedComponentEntry {
                components: self,
                id,
                component,
            }),
            Err(_) => ComponentEntry::Vacant(VacantComponentEntry {
                components: self,
                id,
            }),
        }
    }
}

#[derive(Debug)]
pub enum ComponentEntry<'a> {
    Occupied(OccupiedComponentEntry<'a>),
    Vacant(VacantComponentEntry<'a>),
}

impl<'a> ComponentEntry<'a> {
    pub fn or_default(self) -> RawComponent {
        match self {
            Self::Occupied(entry) => entry.component,
            Self::Vacant(_) => RawComponent::empty(),
        }
    }

    pub fn or_insert_with<F>(self, f: F) -> RawComponent
    where
        F: FnOnce(&mut RawComponent),
    {
        match self {
            Self::Occupied(entry) => entry.component,
            Self::Vacant(_) => {
                let mut component = RawComponent::empty();
                f(&mut component);
                component
            }
        }
    }
}

impl<'a> ComponentEntry<'a> {
    #[inline]
    pub fn key(&self) -> RecordReference {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }
}

#[derive(Debug)]
pub struct OccupiedComponentEntry<'a> {
    components: &'a ItemComponents<'a>,
    id: RecordReference,
    component: RawComponent,
}

impl<'a> OccupiedComponentEntry<'a> {
    #[inline]
    pub fn key(&self) -> RecordReference {
        self.id
    }

    #[inline]
    pub fn get(&self) -> &RawComponent {
        &self.component
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut RawComponent {
        &mut self.component
    }

    pub fn remove(self) -> RawComponent {
        self.components.remove(self.id).unwrap();
        self.component
    }
}

#[derive(Debug)]
pub struct VacantComponentEntry<'a> {
    components: &'a ItemComponents<'a>,
    id: RecordReference,
}

impl<'a> VacantComponentEntry<'a> {
    pub fn insert(self, value: RawComponent) -> RawComponent {
        self.components.insert(self.id, &value).unwrap();
        value
    }

    #[inline]
    pub fn key(&self) -> RecordReference {
        self.id
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Item {
    pub id: RecordReference,
    pub equipped: bool,
    pub hidden: bool,
}

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
    inner: alloc::vec::IntoIter<u64>,
}

impl Iterator for Keys {
    type Item = InventoryId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(InventoryId)
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
pub struct Iter<'a> {
    inventory: &'a Inventory,
    keys: Keys,
}

impl<'a> Iterator for Iter<'a> {
    type Item = ItemStackRef;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys.next()?;
        let stack = self.inventory.get(key).unwrap();
        Some(stack)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.keys.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

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
            item: RawItem {
                id: self.id,
                equipped: 0,
                hdden: 0,
                _pad0: 0,
            },
            quantity: self.quantity,
        };

        let res =
            unsafe { inventory_insert(inventory.entity.into_raw(), &stack, slot_id.as_mut_ptr()) };
        assert!(res == 0);

        let slot_id = unsafe { slot_id.assume_init() };

        for (id, component) in &self.components {
            let res = unsafe {
                inventory_component_insert(
                    inventory.entity.into_raw(),
                    slot_id,
                    &id,
                    component.as_ptr(),
                    component.len(),
                )
            };
            assert!(res == 0);
        }

        InventoryId(slot_id)
    }
}
