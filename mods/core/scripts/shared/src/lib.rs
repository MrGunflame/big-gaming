use std::f32::consts::PI;

pub use game_wasm::math::Vec3;

use components::MOVEMENT_SPEED;
use game_wasm::entity::EntityId;
use game_wasm::math::Quat;
use game_wasm::world::Entity;

/// Updates per second.
// FIXME: Unhardcode this value, it should be provided by the runtime
// to support running the game different update rates.
const UPS: f32 = 60.0;

pub fn extract_actor_rotation(rotation: Quat) -> Quat {
    let mut pt = rotation * -Vec3::Z;

    if pt.y == 1.0 {
        return rotation;
    }

    pt.y = 0.0;
    if !pt.is_normalized() {
        pt = pt.normalize();
    }

    let b = Vec3::Z;

    let mut angle = f32::clamp(pt.dot(b), -1.0, 1.0).acos();
    if pt.x.is_sign_negative() {
        angle = -angle;
    }

    let res = Quat::from_axis_angle(Vec3::Y, angle + PI);
    debug_assert!(!res.is_nan());
    res
}

#[macro_export]
macro_rules! impl_movement {
    ($dir:expr) => {
        #[game_wasm::events::on_action]
        fn on_action(entity: u64, invoker: u64) {
            $crate::on_action_impl(entity, invoker, $dir);
        }
    };
}

#[inline]
pub fn on_action_impl(entity: u64, _invoker: u64, dir: Vec3) {
    let mut entity = Entity::get(EntityId::from_raw(entity)).unwrap();

    let speed: f32 = entity.components().get(MOVEMENT_SPEED).unwrap().read();

    let rotation = extract_actor_rotation(entity.rotation());
    let mut translation = entity.translation();

    translation += rotation * dir * (speed / UPS);
    entity.set_translation(translation);
}

pub mod components {
    use game_wasm::record::{ModuleId, RecordId};
    use game_wasm::world::RecordReference;

    const MODULE: ModuleId = ModuleId::from_str_const("c626b9b0ab1940aba6932ea7726d0175");

    pub const MOVEMENT_SPEED: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(5),
    };
}
