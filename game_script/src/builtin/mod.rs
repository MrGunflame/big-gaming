//! Builtin host functions

mod events;
mod host_buffer;
mod log;
mod physics;
mod player;
mod process;
mod record;
mod system;
mod world;

use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};
use thiserror::Error;
use wasmtime::{Caller, Linker};

use crate::instance::State;

macro_rules! register_fns {
    ($linker:expr, $($id:ident),*$(,)?) => {
        $(
            $linker.func_wrap("host", stringify!($id), $id).unwrap();
        )*
    };
}

pub fn register_host_fns(store: &mut Linker<State>) {
    use events::*;
    use host_buffer::*;
    use log::*;
    use physics::*;
    use player::*;
    use process::*;
    use record::*;
    use system::*;
    use world::*;

    register_fns! {
        store,
        log,
        abort,
        world_entity_spawn,
        world_entity_despawn,
        world_entity_component_len,
        world_entity_component_get,
        world_entity_component_insert,
        world_entity_component_remove,
        physics_cast_ray,
        physics_cast_shape,
        player_lookup,
        player_set_active,
        register_system,
        register_event_handler,
        register_action_handler,
        event_dispatch,
        host_buffer_len,
        host_buffer_get,
        record_data_len,
        record_data_copy,
        prefab_spawn,
        resource_create_runtime,
        resource_destroy_runtime,
        resource_get_runtime,
        resource_len_runtime,
        resource_update_runtime,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("invalid invariant")]
pub struct InvalidInvariant;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("explicit abort")]
pub struct Abort;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("no memory")]
    NoMemory,
    #[error("oob pointer")]
    BadPointer,
}

trait CallerExt<T> {
    /// Splits the `Caller` into the memory of the guest and the data carried by the caller.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the guest does not expose a memory.
    fn split(&mut self) -> wasmtime::Result<(GuestMemory<'_>, &mut T)>;
}

impl<'a, T> CallerExt<T> for Caller<'a, T> {
    fn split(&mut self) -> wasmtime::Result<(GuestMemory<'_>, &mut T)> {
        let memory = self
            .get_export("memory")
            .and_then(|m| m.into_memory())
            .ok_or_else(|| wasmtime::Error::new(Error::NoMemory))?;
        let (memory, data) = memory.data_and_store_mut(self);
        Ok((GuestMemory { memory }, data))
    }
}

trait AsMemory {
    /// Reads `len` bytes of guest memory starting at `ptr`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the read is outside the bounds of the memory.
    ///
    /// [`Error`]: wasmtime::Error
    fn read_memory(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[u8]>;

    /// Writes `buf` into guest memory starting at `ptr`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the write exceeds the bounds of the memory.
    ///
    /// [`Error`]: wasmtime::Error
    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()>;

    /// Read a `T` from the given `ptr`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the memory is too short to read `T` starting from `ptr`.
    ///
    /// [`Error`]: wasmtime::Error
    fn read<T>(&mut self, ptr: u32) -> wasmtime::Result<T>
    where
        T: AnyBitPattern,
    {
        let len = Usize::size_of::<T>();
        let bytes = self.read_memory(ptr, len.0)?;
        Ok(bytemuck::pod_read_unaligned(bytes))
    }

    /// Write a `T` to the given `ptr`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the memory is too short to write `T` starting at `ptr`.
    ///
    /// [`Error`]: wasmtime::Error
    fn write<T>(&mut self, ptr: u32, value: &T) -> wasmtime::Result<()>
    where
        T: Copy + NoUninit,
    {
        self.write_memory(ptr, bytemuck::bytes_of(value))
    }

    /// Read a slice of `len` `T`s from the given `ptr`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the memory is too short to read `len` `T`s starting from `ptr`.
    ///
    /// [`Error`]: wasmtime::Error
    fn read_slice<T>(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[T]>
    where
        T: Copy + AnyBitPattern,
    {
        let bytes = self.read_memory(ptr, len.wrapping_mul(Usize::size_of::<T>().0))?;
        Ok(bytemuck::cast_slice(bytes))
    }
}

impl<'a, S> AsMemory for Caller<'a, S> {
    fn read_memory(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[u8]> {
        let start = Usize(ptr).to_usize();
        let end = start.wrapping_add(Usize(len).to_usize());

        let memory = self
            .get_export("memory")
            .and_then(|m| m.into_memory())
            .ok_or_else(|| wasmtime::Error::new(Error::NoMemory))?;

        let bytes = memory
            .data(self)
            .get(start..end)
            .ok_or_else(|| wasmtime::Error::new(Error::BadPointer))?;

        Ok(bytes)
    }

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()> {
        let start = Usize(ptr).to_usize();
        let end = start.wrapping_add(buf.len());

        let memory = self
            .get_export("memory")
            .and_then(|m| m.into_memory())
            .ok_or_else(|| wasmtime::Error::new(Error::NoMemory))?;

        let bytes = memory
            .data_mut(self)
            .get_mut(start..end)
            .ok_or_else(|| wasmtime::Error::new(Error::BadPointer))?;

        bytes.copy_from_slice(buf);
        Ok(())
    }
}

/// A view into the memory of the guest.
#[derive(Debug)]
pub struct GuestMemory<'a> {
    memory: &'a mut [u8],
}

impl<'a> AsMemory for GuestMemory<'a> {
    fn read_memory(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[u8]> {
        let start = Usize(ptr).to_usize();
        let end = start.wrapping_add(Usize(len).to_usize());

        let bytes = self
            .memory
            .get(start..end)
            .ok_or_else(|| wasmtime::Error::new(Error::BadPointer))?;

        Ok(bytes)
    }

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()> {
        let start = Usize(ptr).to_usize();
        let end = start.wrapping_add(buf.len());

        let bytes = self
            .memory
            .get_mut(start..end)
            .ok_or_else(|| wasmtime::Error::new(Error::BadPointer))?;

        bytes.copy_from_slice(buf);
        Ok(())
    }
}

macro_rules! log_fn_invocation {
    ($name:expr, $($input:expr),* $(,)? $(=> $output:expr)?) => {{
        use core::fmt::Write as _;

        let mut _buffer = std::string::String::from($name);

        {
            _buffer.push('(');
            let mut _count = 0;
            $(
                let _ = core::write!(_buffer, "{} = {:?},", stringify!($input), $input);
                _count += 1;
            )*

            // Strip trailing comma
            if _count != 0 {
                _buffer.truncate(_buffer.len() - 1);
            }
            _buffer.push(')');
        }

        {
            $(
                let _ = core::write!(_buffer, " = {:?}", $output);
            )*
        }

        tracing::trace!("{}", _buffer);
    }};
}

/// Emits a warning if the precondition is not satisfied.
macro_rules! assert_caller_precondition {
    ($name:expr, $precondition:expr) => {
        // TODO: Add cfg to disable precondition checks.
        if !$precondition {
            tracing::warn!(
                "{} precondition violated: {}",
                $name,
                stringify!($precondition)
            );
        }
    };
}

pub(crate) use {assert_caller_precondition, log_fn_invocation};

/// A `usize`-sized type for the guest.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
#[repr(transparent)]
struct Usize(u32);

impl Usize {
    const MAX_USIZE: usize = u32::MAX as usize;

    #[allow(clippy::cast_possible_truncation)]
    fn new_unchecked(value: usize) -> Self {
        Self(value as u32)
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn to_usize(self) -> usize {
        const _: () = assert!(usize::BITS >= u32::BITS);
        // This cast will always suceed if the above assertions
        // is true.
        self.0 as usize
    }

    #[allow(clippy::cast_possible_truncation)]
    const fn size_of<T>() -> Self {
        const { assert!(size_of::<T>() <= Self::MAX_USIZE) }
        Self(size_of::<T>() as u32)
    }
}
