//! WASM host bindings
#![no_std]

use core::ffi::c_void;
use core::fmt::{self, Display, Formatter};

use entity::EntityId;
use inventory::InventoryId;
use world::RecordReference;

extern crate alloc;

// Derive macro hackery, allowing us to derive in the crate
// the types are defined in.
extern crate self as game_wasm;

#[cfg(test)]
extern crate std;

#[cfg(feature = "raw")]
pub mod raw;
#[cfg(not(feature = "raw"))]
mod raw;

pub mod action;
pub mod components;
pub mod entity;
pub mod events;
pub mod inventory;
pub mod log;
pub mod math;
pub mod physics;
pub mod player;
pub mod process;
pub mod record;
pub mod system;
pub mod world;

/// The error type returned by failed operations.
#[derive(Clone, Debug)]
pub struct Error(ErrorImpl);

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ErrorImpl {
    NoEntity(EntityId),
    NoComponent(RecordReference),
    NoInventorySlot(InventoryId),
    ComponentDecode,
}

impl ErrorImpl {
    #[inline]
    pub(crate) const fn into_error(self) -> Error {
        Error(self)
    }
}

impl Display for ErrorImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEntity(id) => write!(f, "no such entity: {:?}", id),
            Self::NoComponent(id) => write!(f, "no component: {:?}", id),
            Self::NoInventorySlot(id) => write!(f, "no inventory slot id: {:?}", id),
            Self::ComponentDecode => write!(f, "component decode failed"),
        }
    }
}

#[cfg(all(feature = "global_alloc", target_arch = "wasm32"))]
#[global_allocator]
static DLMALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

/// Hint to the compiler that the function is never reachable.
///
/// This function should be prefered over [`core::hint::unreachable_unchecked`] because it
/// panics when `debug_assertions` is enabled instead of emitting UB.
///
/// # Safety
///
/// This function must never be called.
#[inline]
#[cfg_attr(any(debug_assertions, miri), track_caller)]
pub(crate) const unsafe fn unreachable_unchecked() -> ! {
    if cfg!(debug_assertions) {
        core::unreachable!();
    } else {
        // SAFETY: The caller guarantees that this call site is never reached.
        unsafe { core::hint::unreachable_unchecked() }
    }
}

#[no_mangle]
extern "C" fn __wasm_fn_trampoline(ptr: *const (), entity: u64) {
    unsafe {
        let ptr = core::mem::transmute::<*const (), unsafe fn(EntityId, c_void)>(ptr);
        let vtable = system::SYSTEM_PTRS.get(ptr as usize);
        (vtable.run)(EntityId::from_raw(entity), ptr);
    }
}
