use game_common::entity::EntityId;
use game_tracing::trace_span;
use game_wasm::player::PlayerId;
use wasmtime::{Caller, Result};

use crate::instance::State;

use super::CallerExt;

pub(super) fn player_lookup(
    mut caller: Caller<'_, State<'_>>,
    entity_id: u64,
    out: u32,
) -> Result<u32> {
    let _span = trace_span!("player_lookup").entered();

    let entity = EntityId::from_raw(entity_id);

    let Some(player) = caller.data_mut().player_lookup(entity) else {
        return Ok(1);
    };

    caller.write(out, &player.to_bits())?;
    Ok(0)
}

pub(super) fn player_set_active(
    mut caller: Caller<'_, State<'_>>,
    player_id: u64,
    entity_id: u64,
) -> Result<u32> {
    let _span = trace_span!("player_set_active").entered();

    let player = PlayerId::from_raw(player_id);
    let entity = EntityId::from_raw(entity_id);

    if !caller.data_mut().player_set_active(player, entity) {
        return Ok(1);
    }

    Ok(0)
}
