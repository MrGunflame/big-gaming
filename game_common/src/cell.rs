use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// A cell for unchecked interior mutability.
///
/// This is effectively a [`UnsafeCell`] with the same safety requirements, but with the following
/// differences:
/// - `UnsafeRefCell` is **NOT** `#[repr(transparent)]`. The memory layout is not defined.
/// - `UnsafeRefCell` has extra assertions via RAII when `debug_assertions` are enabled.
/// - `UnsafeRefCell` returns references instead of raw pointers.
#[derive(Debug, Default)]
pub struct UnsafeRefCell<T>
where
    T: ?Sized,
{
    #[cfg(debug_assertions)]
    state: RwLock<()>,
    cell: UnsafeCell<T>,
}

impl<T> UnsafeRefCell<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            state: RwLock::new(()),
            cell: UnsafeCell::new(value),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }
}

impl<T> UnsafeRefCell<T>
where
    T: ?Sized,
{
    /// Returns a immutable reference to the value.
    ///
    /// # Safety
    ///
    /// For the duration of the borrow, no other mutable references may exist.
    #[inline]
    #[track_caller]
    pub unsafe fn get(&self) -> Ref<'_, T> {
        #[cfg(debug_assertions)]
        let Some(guard) = self.state.try_read() else {
            panic!("UnsafeRefCell is already borrowed");
        };

        // SAFETY: The caller guarantees that there are no mutable references
        // for the duration of this borrow.
        let value = unsafe { &*self.cell.get() };
        Ref {
            _guard: guard,
            value,
        }
    }

    /// Returns a mutable reference to the value.
    ///
    /// # Safety
    ///
    /// For the duration of the borrow, no other references (mutable or immutable) may exist.
    #[inline]
    #[track_caller]
    pub unsafe fn get_mut(&self) -> RefMut<'_, T> {
        #[cfg(debug_assertions)]
        let Some(guard) = self.state.try_write() else {
            panic!("UnsafeRefCell is already borrowed");
        };

        // SAFETY: The caller guarantees that there are no other references
        // for the duration of this borrow.
        let value = unsafe { &mut *self.cell.get() };
        RefMut {
            value,
            _guard: guard,
        }
    }
}

/// An immutable reference to a value in a [`UnsafeRefCell`].
#[derive(Debug)]
pub struct Ref<'a, T>
where
    T: ?Sized,
{
    value: &'a T,
    #[cfg(debug_assertions)]
    _guard: RwLockReadGuard<'a, ()>,
}

impl<'a, T> Deref for Ref<'a, T>
where
    T: ?Sized,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// A mutable reference to a value in a [`UnsafeRefCell`].
#[derive(Debug)]
pub struct RefMut<'a, T>
where
    T: ?Sized,
{
    value: &'a mut T,
    #[cfg(debug_assertions)]
    _guard: RwLockWriteGuard<'a, ()>,
}

impl<'a, T> Deref for RefMut<'a, T>
where
    T: ?Sized,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T> DerefMut for RefMut<'a, T>
where
    T: ?Sized,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}
