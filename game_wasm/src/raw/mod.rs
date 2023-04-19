pub mod process;
pub mod record;
pub mod world;

use core::marker::PhantomData;
use core::ptr::NonNull;

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

#[link(wasm_import_module = "host")]
extern "C" {
    pub fn log(level: u32, ptr: Usize, len: Usize);
}
