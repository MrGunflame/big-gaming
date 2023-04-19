use wasmtime::{Caller, Error};

use crate::instance::State;

use super::Abort;

pub fn abort(mut caller: Caller<'_, State<'_>>) -> wasmtime::Result<()> {
    Err(Error::new(Abort))
}
