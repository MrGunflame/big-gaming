use game_common::entity::EntityId;
use game_common::world::entity::{EntityBody, EntityKind};
use game_wasm::raw;
use game_wasm::raw::record::RecordReference;
use game_wasm::raw::world::{Entity, Item};
use wasmtime::{Caller, Result, WasmTy};

use crate::abi::ToAbi;
use crate::instance::State;

use super::CallerExt;

pub fn world_entity_spawn(mut caller: Caller<'_, State<'_>>, ptr: u32) -> Result<u32> {
    let entity: Entity = caller.read(ptr)?;

    todo!()
}

pub fn world_entity_get(mut caller: Caller<'_, State<'_>>, id: u64, out: u32) -> Result<u32> {
    let Some(entity) = caller.data_mut().world.get(EntityId::from_raw(id)) else {
       return Ok(1);
    };

    let entity = entity.to_abi();

    caller.write(out, &entity);
    Ok(0)
}

pub fn world_entity_despawn(mut caller: Caller<'_, State<'_>>, id: u64) -> Result<u32> {
    let id = EntityId::from_raw(id);

    caller.data_mut().world.despawn(id);
    Ok(0)
}

pub fn world_entity_component_get(mut caller: Caller<'_, State<'_>>) -> Result<u32> {
    todo!()
}

pub fn world_entity_component_insert(mut caller: Caller<'_, State<'_>>) -> Result<u32> {
    todo!()
}
