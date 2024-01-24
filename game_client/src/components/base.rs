use game_common::components::{Decode, Encode};
use game_wasm::components::Component;
use game_wasm::record::{ModuleId, RecordId};
use game_wasm::world::RecordReference;

const MODULE: ModuleId = ModuleId::from_str_const("c626b9b0ab1940aba6932ea7726d0175");

const HEALTH: RecordReference = RecordReference {
    module: MODULE,
    record: RecordId(0x13),
};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Health {
    pub value: u32,
    pub max: u32,
}

impl Component for Health {
    const ID: RecordReference = HEALTH;
}
