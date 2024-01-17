use bytemuck::{Pod, Zeroable};
use game_common::entity::EntityId;
use game_tracing::trace_span;
use game_wasm::world::RecordReference;
use wasmtime::{Caller, Result};

use crate::events::{DispatchEvent, Receiver};
use crate::instance::State;

use super::CallerExt;

pub fn event_dispatch(
    mut caller: Caller<'_, State>,
    id: u32,
    rx: u32,
    ptr: u32,
    len: u32,
) -> Result<()> {
    let _span = trace_span!("event_dispatch").entered();

    let id: RecordReference = caller.read(id)?;
    let rx: EventReceiver = caller.read(rx)?;
    let data = caller.read_memory(ptr, len)?.to_vec();

    let receiver = match rx.entities_ptr {
        0 => Receiver::All,
        _ => {
            let entities: &[EntityId] = caller.read_slice(rx.entities_ptr, rx.entities_len)?;
            Receiver::Entities(entities.to_vec())
        }
    };

    caller
        .data_mut()
        .as_run_mut()?
        .events
        .push(DispatchEvent { id, receiver, data });

    Ok(())
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct EventReceiver {
    entities_ptr: u32,
    entities_len: u32,
}
