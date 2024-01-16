use bytemuck::{Pod, Zeroable};
use game_wasm::world::RecordReference;
use wasmtime::{Caller, Result};

use crate::instance::State;
use crate::{Entry, Pointer, System, SystemQuery};

use super::CallerExt;

pub fn register_system(mut caller: Caller<'_, State<'_>>, params: u32, fn_ptr: u32) -> Result<()> {
    let params: SystemParams = caller.read(params)?;

    let query = caller
        .read_slice(params.query_components_ptr, params.query_components_len)?
        .to_vec();

    let state = caller.data_mut().as_init()?;
    state.systems.push(System {
        script: state.script,
        ptr: Pointer(fn_ptr),
        query: SystemQuery { components: query },
    });

    Ok(())
}

pub fn register_event_handler(mut caller: Caller<'_, State<'_>>, fn_ptr: u32) -> Result<()> {
    todo!()
}

pub fn register_action_handler(
    mut caller: Caller<'_, State<'_>>,
    id: u32,
    fn_ptr: u32,
) -> Result<()> {
    let id: RecordReference = caller.read(id)?;

    let state = caller.data_mut().as_init()?;
    state.actions.entry(id).or_default().push(Entry {
        script: state.script,
        fn_ptr: Pointer(fn_ptr),
    });

    Ok(())
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct SystemParams {
    query_components_ptr: u32,
    query_components_len: u32,
}
