use bytemuck::{Pod, Zeroable};
use wasmtime::{Caller, Result};

use crate::instance::State;
use crate::{System, SystemQuery};

use super::CallerExt;

pub fn register_system(mut caller: Caller<'_, State<'_>>, params: u32, fn_ptr: u32) -> Result<()> {
    let params: SystemParams = caller.read(params)?;

    let mut query = Vec::new();
    for index in 0..params.query_components_len {
        let elem = caller.read(params.query_components_ptr.wrapping_add(index))?;
        query.push(elem);
    }

    let state = caller.data_mut().as_init()?;
    state.systems.push(System {
        script: state.script,
        ptr: fn_ptr,
        query: SystemQuery { components: query },
    });

    Ok(())
}

pub fn register_event_handler(mut caller: Caller<'_, State<'_>>, fn_ptr: u32) -> Result<()> {
    todo!()
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct SystemParams {
    query_components_ptr: u32,
    query_components_len: u32,
}
