pub mod components;
pub mod health;
pub mod inventory;
pub mod physics;
pub mod process;
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
}

/// The entity does not exist.
pub const ERROR_NO_ENTITY: u32 = 1;

/// The component does not exist on the entity.
pub const ERROR_NO_COMPONENT: u32 = 2;

// #[cfg(target_arch = "wasm32")]
// #[link(wasm_import_module = "host")]
// extern "C" {
//     pub fn log(level: u32, ptr: Usize, len: Usize);
// }

// #[cfg(not(target_arch = "wasm32"))]
// pub unsafe extern "C" fn log(level: u32, ptr: Usize, len: Usize) {
//     let _ = (level, ptr, len);
//     panic!("`log` is not implemented on this target");
// }

#[guest_only]
pub fn log(level: u32, ptr: Usize, len: Usize);
