use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::mem;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::vec::Vec;

use crate::action::{Action, ActionBuffer};
use crate::entity::EntityId;
use crate::events::Event;
use crate::raw::Query as RawQuery;
use crate::record::RecordReference;

pub(crate) static SYSTEM_PTRS: SystemPointers = SystemPointers::new();

pub fn register_system(query: Query, f: fn(EntityId)) {
    let fn_ptr = f as *const unsafe fn(c_void);

    unsafe fn run_impl(entity: EntityId, f: unsafe fn(EntityId, c_void)) {
        (unsafe { mem::transmute::<unsafe fn(EntityId, c_void), fn(EntityId)>(f) })(entity);
    }

    let vtable = Vtable { run: run_impl };
    SYSTEM_PTRS.insert(fn_ptr as usize, vtable);

    let raw_query = RawQuery {
        components_ptr: query.components.as_ptr(),
        components_len: query.components.len(),
    };

    unsafe {
        crate::raw::register_system(&raw_query, fn_ptr.cast());
    }
}

#[derive(Debug)]
pub struct Query {
    pub components: Vec<RecordReference>,
}

// NOTE: The EntityId does nothing currently.
pub fn register_event_handler<T>(f: fn(EntityId, T))
where
    T: Event,
{
    let fn_ptr = f as *const unsafe fn(c_void);

    unsafe fn run_impl<T>(entity: EntityId, f: unsafe fn(EntityId, c_void))
    where
        T: Event,
    {
        if let Ok(event) = ActionBuffer::load().get() {
            (unsafe { mem::transmute::<unsafe fn(EntityId, c_void), fn(EntityId, T)>(f) })(
                entity, event,
            );
        }
    }

    let vtable = Vtable { run: run_impl::<T> };
    SYSTEM_PTRS.insert(fn_ptr as usize, vtable);

    unsafe {
        crate::raw::register_event_handler(&T::ID, fn_ptr.cast());
    }
}

pub fn register_action_handler<T>(f: fn(EntityId, T))
where
    T: Action,
{
    let fn_ptr = f as *const unsafe fn(EntityId, c_void);

    unsafe fn run_impl<T>(entity: EntityId, f: unsafe fn(EntityId, c_void))
    where
        T: Action,
    {
        if let Ok(action) = ActionBuffer::load().get() {
            (unsafe { mem::transmute::<unsafe fn(EntityId, c_void), fn(EntityId, T)>(f) })(
                entity, action,
            );
        }
    }

    let vtable = Vtable { run: run_impl::<T> };
    SYSTEM_PTRS.insert(fn_ptr as usize, vtable);

    unsafe {
        crate::raw::register_action_handler(&T::ID, fn_ptr.cast());
    }
}

pub(crate) struct SystemPointers {
    ptrs: UnsafeCell<Vec<(usize, Vtable)>>,
    // Not actually necessary for wasm32-unknown-unknown which is only
    // single-threaded with our exposed host functions.
    flags: AtomicUsize,
}

impl SystemPointers {
    const fn new() -> Self {
        Self {
            ptrs: UnsafeCell::new(Vec::new()),
            flags: AtomicUsize::new(0),
        }
    }

    fn insert(&self, ptr: usize, vtable: Vtable) {
        self.lock_writeable();
        let ptrs = unsafe { &mut *self.ptrs.get() };
        ptrs.push((ptr, vtable));
        self.unlock_writetable();
    }

    /// # Safety
    ///
    /// `ptr` must be contained.
    pub(crate) unsafe fn get(&self, ptr: usize) -> Vtable {
        self.lock_readable();
        let ptrs = unsafe { &*self.ptrs.get() };

        for (p, vtable) in ptrs {
            if *p == ptr {
                self.unlock_readable();
                return *vtable;
            }
        }

        unsafe {
            crate::unreachable_unchecked();
        }
    }

    fn lock_writeable(&self) {
        while self
            .flags
            .compare_exchange_weak(0, usize::MAX, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn unlock_writetable(&self) {
        self.flags.store(0, Ordering::Release);
    }

    fn lock_readable(&self) {
        loop {
            let flags = self.flags.load(Ordering::Acquire);
            let new_flags = flags + 1;
            if new_flags == usize::MAX {
                game_wasm::process::abort();
            }

            if self
                .flags
                .compare_exchange_weak(flags, new_flags, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    fn unlock_readable(&self) {
        self.flags.fetch_sub(1, Ordering::Release);
    }
}

unsafe impl Send for SystemPointers {}
unsafe impl Sync for SystemPointers {}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Vtable {
    pub(crate) run: unsafe fn(EntityId, unsafe fn(EntityId, c_void)),
}
