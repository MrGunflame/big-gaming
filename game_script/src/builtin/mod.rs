//! Builtin host functions

mod log;
mod process;
mod world;

use std::mem;

use bytemuck::{AnyBitPattern, NoUninit};
use thiserror::Error;
use wasmtime::{Caller, Linker, Store};

use crate::instance::State;

macro_rules! register_fns {
    ($linker:expr, $($id:ident),*$(,)?) => {
        $(
            $linker.func_wrap("host", stringify!($id), $id);
        )*
    };
}

pub fn register_host_fns(store: &mut Linker<State>) {
    use log::*;
    use process::*;
    use world::*;

    register_fns! {
       store,
       log,
       abort,
       world_entity_get,
       world_entity_spawn,
       world_entity_despawn,
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
    #[error("invalid invariant")]
    InvalidInvariant,
}

trait CallerExt {
    fn read_memory<'s>(&'s mut self, ptr: u32, len: u32) -> wasmtime::Result<&'s [u8]>;

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
    fn read_memory<'s>(&'s mut self, ptr: u32, len: u32) -> wasmtime::Result<&'s [u8]> {
        let memory = self.get_export("memory").unwrap().into_memory().unwrap();
        let bytes = memory
            .data(self)
            .get(ptr as usize..ptr as usize + len as usize)
            .unwrap();

        Ok(bytes)
    }

    fn write_memory(&mut self, ptr: u32, buf: &[u8]) -> wasmtime::Result<()> {
        let memory = self.get_export("memory").unwrap().into_memory().unwrap();
        let mut bytes = memory
            .data_mut(self)
            .get_mut(ptr as usize..ptr as usize + buf.len())
            .unwrap();

        bytes.copy_from_slice(buf);
        Ok(())
    }
}
