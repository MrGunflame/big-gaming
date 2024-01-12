pub mod components;
pub mod health;
pub mod inventory;
pub mod physics;
pub mod process;
pub mod record;
pub mod world;

use core::marker::PhantomData;
use core::ptr::NonNull;

use game_macros::guest_only;

/// The pointer-sized type.
///
/// For `wasm32-unknown-unknown` this is equivalent to `usize`.
pub type Usize = u32;

/// A transparent pointer type.
///
/// Note that `Ptr` is guaranteed to have the same ABI as [`Usize`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ptr<T> {
    ptr: Usize,
    _marker: PhantomData<*const T>,
}

impl<T> Ptr<T> {
    #[inline]
    pub fn dangling() -> Self {
        let ptr = NonNull::<T>::dangling().as_ptr() as Usize;

        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn from_raw(ptr: Usize) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn from_ptr(ptr: *const T) -> Self {
        Self {
            ptr: ptr as Usize,
            _marker: PhantomData,
        }
    }
}

/// A transparent mutable pointer type.
///
/// Note that `PtrMut` is guaranteed to have the same ABI as [`Usize`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PtrMut<T> {
    ptr: Usize,
    _marker: PhantomData<*mut T>,
}

impl<T> PtrMut<T> {
    #[inline]
    pub fn dangling() -> Self {
        let ptr = NonNull::<T>::dangling().as_ptr() as Usize;

        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn from_raw(ptr: Usize) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn from_ptr(ptr: *mut T) -> Self {
        Self {
            ptr: ptr as Usize,
            _marker: PhantomData,
        }
    }
}

pub const RESULT_OK: u32 = 0;
pub const RESULT_NO_ENTITY: u32 = 1;
pub const RESULT_NO_COMPONENT: u32 = 2;
pub const RESULT_NO_INVENTORY_SLOT: u32 = 3;

#[guest_only]
pub fn log(level: u32, ptr: Usize, len: Usize);

#[guest_only]
pub fn player_lookup(entity_id: u64, player_id: *mut u64) -> u32;

#[guest_only]
pub fn player_set_active(player_id: u64, entity_id: u64) -> u32;
