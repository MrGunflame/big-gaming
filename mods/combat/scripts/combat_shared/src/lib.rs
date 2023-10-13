use bytemuck::{Pod, Zeroable};
use game_wasm::world::RecordReference;

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct GunProperties {
    /// Damage multiplier.
    pub damage: f32,
    /// The cooldown between each shot.
    ///
    /// Maps inversely to rate of fire.
    pub cooldown: f32,
    /// The maximum number of rounds in the magazine.
    pub magazine_capacity: u32,
    /// The object id of the projectile that is being fired.
    pub projectile: RecordReference,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct Ammo(pub u32);

impl Ammo {
    pub fn try_decrement(&mut self) -> bool {
        self.0.checked_sub(1).is_some()
    }
}

pub mod components {
    use game_wasm::record::{ModuleId, RecordId};
    use game_wasm::world::RecordReference;

    pub const GUN_PROPERTIES: RecordReference = RecordReference {
        module: ModuleId::from_str_const("848b1d80bfc74af3ae27d922c5da238d"),
        record: RecordId(04),
    };

    pub const AMMO: RecordReference = RecordReference {
        module: ModuleId::from_str_const("848b1d80bfc74af3ae27d922c5da238d"),
        record: RecordId(05),
    };
}
