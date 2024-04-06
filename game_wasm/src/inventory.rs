use core::iter::FusedIterator;

use alloc::vec::Vec;

use crate::components::builtin::INVENTORY;
use crate::components::{Component, Components};
use crate::encoding::{Decode, Encode, Reader, Writer};
use crate::{Error, ErrorImpl};

use crate::world::RecordReference;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct InventorySlotId(u64);

#[derive(Clone, Debug)]
pub struct Inventory {
    items: Vec<ItemStack>,
}

impl Inventory {
    /// Creates a new, empty `Inventory`.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Returns the number of [`ItemStack`]s in the `Inventory`.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the `Inventory` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter { items: &self.items }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            inner: self.items.iter_mut(),
        }
    }

    pub fn insert(&mut self, stack: ItemStack) -> InventorySlotId {
        let id = self.items.len();
        self.items.push(stack);
        InventorySlotId(id as u64)
    }

    pub fn remove(&mut self, id: InventorySlotId) -> Option<ItemStack> {
        let index = id.0 as usize;

        if index > self.items.len() {
            None
        } else {
            Some(self.items.remove(id.0 as usize))
        }
    }

    pub fn get(&self, id: InventorySlotId) -> Option<&ItemStack> {
        self.items.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: InventorySlotId) -> Option<&mut ItemStack> {
        self.items.get_mut(id.0 as usize)
    }
}

impl Default for Inventory {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Encode for Inventory {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        for stack in &self.items {
            stack.encode(&mut writer);
        }
    }
}

impl Decode for Inventory {
    type Error = Error;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        let mut items = Vec::new();

        while !reader.chunk().is_empty() {
            let stack =
                ItemStack::decode(&mut reader).map_err(|err| Error(ErrorImpl::ComponentDecode))?;

            items.push(stack);
        }

        Ok(Self { items })
    }
}

impl Component for Inventory {
    const ID: RecordReference = INVENTORY;
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ItemStack {
    pub item: RecordReference,
    pub quantity: u64,
    pub equipped: bool,
    pub hidden: bool,
    pub components: Components,
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = &'a ItemStack;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An `Iterator` over all [`ItemStack`]s in a [`Inventory`].
///
/// Returned by [`iter`].
///
/// [`iter`]: Inventory::iter
#[derive(Clone, Debug)]
pub struct Iter<'a> {
    items: &'a [ItemStack],
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a ItemStack;

    fn next(&mut self) -> Option<Self::Item> {
        let (lhs, rhs) = self.items.split_first()?;
        self.items = rhs;
        Some(lhs)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items.len(), Some(self.items.len()))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

pub struct IterMut<'a> {
    inner: core::slice::IterMut<'a, ItemStack>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = &'a mut ItemStack;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
