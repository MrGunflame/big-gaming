use game_wasm::components::builtin::Transform;
use game_wasm::components::Component;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::math::{Quat, Vec3};
use game_wasm::world::{Entity, RecordReference};

use crate::components::HEALTH;

#[derive(Copy, Clone, Debug, PartialEq, Encode, Decode)]
pub struct Health {
    pub value: u32,
    pub max: u32,
}

impl Component for Health {
    const ID: RecordReference = HEALTH;
}

pub fn apply_actor_damage(damage: u32, target: Entity) {
    let Ok(mut health) = target.get::<Health>() else {
        return;
    };

    health.value = health.value.saturating_sub(damage);

    if health.value != 0 {
        target.insert(health);
        return;
    }

    target.remove::<Health>();

    let mut transform = target.get::<Transform>().unwrap();
    transform.rotation *= Quat::from_axis_angle(Vec3::X, 90.0f32.to_radians());
    target.insert(transform);
}
