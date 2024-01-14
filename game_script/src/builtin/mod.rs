//! Builtin host functions

mod action;
mod inventory;
mod log;
mod physics;
mod player;
mod process;
mod record;
mod world;

use std::mem;

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

pub fn register_host_fns(store: &mut Linker<State<'_>>) {
    use action::*;
    use inventory::*;
    use log::*;
    use physics::*;
    use player::*;
    use process::*;
    use record::*;
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
        inventory_get,
        inventory_insert,
        inventory_remove,
        inventory_component_len,
        inventory_component_get,
        inventory_component_insert,
        inventory_component_remove,
        inventory_equip,
        inventory_unequip,
        inventory_clear,
        inventory_len,
        inventory_list,
        physics_cast_ray,
        physics_cast_shape,
        get_record,
        get_record_len_component,
        get_record_component_keys,
        get_record_component_len,
        get_record_component_get,
        player_lookup,
        player_set_active,
        action_data_buffer_len,
        action_data_buffer_get,
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

trait CallerExt {
    fn read_memory(&mut self, ptr: u32, len: u32) -> wasmtime::Result<&[u8]>;

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()>;

    fn read<T>(&mut self, ptr: u32) -> wasmtime::Result<T>
    where
        T: Copy + AnyBitPattern,
    {
        let len = mem::size_of::<T>();
        let bytes = self.read_memory(ptr, len as u32)?;
        Ok(bytemuck::pod_read_unaligned(bytes))
    }

    fn write<T>(&mut self, ptr: u32, value: &T) -> wasmtime::Result<()>
    where
        T: Copy + NoUninit,
    {
        self.write_memory(ptr, bytemuck::bytes_of(value))
    }
}

impl<'a, S> CallerExt for Caller<'a, S> {
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
