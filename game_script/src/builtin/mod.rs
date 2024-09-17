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

use bytemuck::{AnyBitPattern, NoUninit};
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
    fn read_memory(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[u8]>;

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()>;

    fn read<T>(&mut self, ptr: u32) -> wasmtime::Result<T>
    where
        T: Copy + AnyBitPattern,
    {
        let len = size_of::<T>();
        let bytes = self.read_memory(ptr, len as u32)?;
        Ok(bytemuck::pod_read_unaligned(bytes))
    }

    fn write<T>(&mut self, ptr: u32, value: &T) -> wasmtime::Result<()>
    where
        T: Copy + NoUninit,
    {
        self.write_memory(ptr, bytemuck::bytes_of(value))
    }

    fn read_slice<T>(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[T]>
    where
        T: Copy + AnyBitPattern,
    {
        let bytes = self.read_memory(ptr, len.wrapping_mul(size_of::<T>() as u32))?;
        Ok(bytemuck::cast_slice(bytes))
    }
}

impl<'a, S> AsMemory for Caller<'a, S> {
    fn read_memory(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[u8]> {
        let memory = self
            .get_export("memory")
            .and_then(|m| m.into_memory())
            .ok_or_else(|| wasmtime::Error::new(Error::NoMemory))?;

        let bytes = memory
            .data(self)
            .get(ptr as usize..ptr as usize + len as usize)
            .ok_or_else(|| wasmtime::Error::new(Error::BadPointer))?;

        Ok(bytes)
    }

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()> {
        let memory = self
            .get_export("memory")
            .and_then(|m| m.into_memory())
            .ok_or_else(|| wasmtime::Error::new(Error::NoMemory))?;

        let bytes = memory
            .data_mut(self)
            .get_mut(ptr as usize..ptr as usize + buf.len())
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
        let bytes = self
            .memory
            .get(ptr as usize..ptr as usize + len as usize)
            .ok_or_else(|| wasmtime::Error::new(Error::BadPointer))?;

        Ok(bytes)
    }

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()> {
        let bytes = self
            .memory
            .get_mut(ptr as usize..ptr as usize + buf.len())
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
