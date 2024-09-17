use game_tracing::trace_span;
use game_wasm::log::Level;
use wasmtime::{Caller, Error};

use crate::instance::State;

use super::{AsMemory, InvalidInvariant};

pub fn log(mut caller: Caller<'_, State>, level: u32, ptr: u32, len: u32) -> wasmtime::Result<()> {
    let _span = trace_span!("log").entered();
    tracing::trace!("log(level = {}, ptr = {}, len = {})", level, ptr, len);

    let bytes = caller.read_memory(ptr, len)?;

    let content = std::str::from_utf8(bytes).map_err(|_| Error::new(InvalidInvariant))?;

    match Level::from_raw(level) {
        Level::ERROR => {
            tracing::error!("{}", content);
        }
        Level::WARN => {
            tracing::warn!("{}", content);
        }
        Level::INFO => {
            tracing::info!("{}", content);
        }
        Level::DEBUG => {
            tracing::debug!("{}", content);
        }
        Level::TRACE => {
            tracing::trace!("{}", content);
        }
        _ => return Err(Error::new(InvalidInvariant)),
    }

    Ok(())
}
