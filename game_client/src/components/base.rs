use game_common::components::{Decode, Encode};
use game_wasm::components::Component;
use game_wasm::entity::EntityId;
use game_wasm::record::{ModuleId, RecordId};
use game_wasm::world::RecordReference;

const MODULE: ModuleId = ModuleId::from_str_const("c626b9b0ab1940aba6932ea7726d0175");

const HEALTH: RecordReference = RecordReference {
    module: MODULE,
    record: RecordId(0x13),
};

const CAMERA: RecordReference = RecordReference {
    module: MODULE,
    record: RecordId(0x21),
};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Health {
    pub value: u32,
    pub max: u32,
}

impl Component for Health {
    const ID: RecordReference = HEALTH;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Camera {
    pub parent: EntityId,
}

impl Component for Camera {
    const ID: RecordReference = CAMERA;
}
