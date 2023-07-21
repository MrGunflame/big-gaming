use game_wasm::log::Level;
use wasmtime::{Caller, Error};

use crate::instance::State;

use super::{CallerExt, InvalidInvariant};

pub fn log(
    mut caller: Caller<'_, State<'_, '_>>,
    level: u32,
    ptr: u32,
    len: u32,
) -> wasmtime::Result<()> {
    tracing::trace!("log(level = {}, ptr = {}, len = {})", level, ptr, len);

    let bytes = caller.read_memory(ptr, len)?;

    let content = std::str::from_utf8(bytes).unwrap();

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
