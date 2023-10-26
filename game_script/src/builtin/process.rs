use wasmtime::{Caller, Error};

use crate::instance::State;

use super::Abort;

pub fn abort(_caller: Caller<'_, State<'_>>) -> wasmtime::Result<()> {
    tracing::trace!("abort");

    Err(Error::new(Abort))
}
