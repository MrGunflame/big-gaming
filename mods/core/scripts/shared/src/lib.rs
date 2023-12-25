#![no_std]

extern crate alloc;

use core::f32::consts::PI;

use game_wasm::components::builtin::Transform;
use game_wasm::components::AsComponent;
use game_wasm::math::Real;
pub use game_wasm::math::Vec3;

use bytemuck::{Pod, Zeroable};
use components::MOVEMENT_SPEED;
use game_wasm::entity::EntityId;
use game_wasm::math::Quat;
use game_wasm::world::Entity;
use game_wasm::world::RecordReference;

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
        $crate::panic_handler!();

        #[game_wasm::events::on_action]
        fn on_action(invoker: game_wasm::entity::EntityId) {
            $crate::on_action_impl(invoker, $dir);
        }
    };
}

#[inline]
pub fn on_action_impl(entity: EntityId, dir: Vec3) {
    let entity = Entity::new(entity);

    let speed = entity.get::<MovementSpeed>();
    let mut transform = entity.get::<Transform>();

    let rotation = extract_actor_rotation(transform.rotation);

    transform.translation += rotation * dir * (speed.0 / UPS);

    entity.insert(transform);
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct MovementSpeed(pub f32);

impl AsComponent for MovementSpeed {
    const ID: RecordReference = MOVEMENT_SPEED;

    fn from_bytes(buf: &[u8]) -> Self {
        bytemuck::pod_read_unaligned(buf)
    }

    fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

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
    pub projectile: Projectile,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Projectile {
    /// The object id of the projectile that is being fired.
    pub id: RecordReference,
    // FIXME: This is an array so we don't have to bother with
    // alignment.
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct Ammo(pub u32);

impl Ammo {
    #[inline]
    pub fn try_decrement(&mut self) -> bool {
        match self.0.checked_sub(1) {
            Some(val) => {
                self.0 = val;
                true
            }
            None => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct Health(pub f32);

impl AsComponent for Health {
    const ID: RecordReference = components::HEALTH;

    fn from_bytes(buf: &[u8]) -> Self {
        let v = bytemuck::pod_read_unaligned(buf);
        Self(v)
    }

    fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct ProjectileProperties {
    pub damage: f32,
}

impl AsComponent for ProjectileProperties {
    const ID: RecordReference = components::PROJECTILE_PROPERTIES;

    fn from_bytes(buf: &[u8]) -> Self {
        let damage = bytemuck::pod_read_unaligned(buf);
        Self { damage }
    }

    fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

pub mod components {
    use game_wasm::record::{ModuleId, RecordId};
    use game_wasm::world::RecordReference;

    const MODULE: ModuleId = ModuleId::from_str_const("c626b9b0ab1940aba6932ea7726d0175");

    pub const MOVEMENT_SPEED: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(5),
    };

    pub const GUN_PROPERTIES: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(0xb),
    };

    pub const AMMO: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(0xc),
    };

    pub const HEALTH: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(0x13),
    };

    pub const PROJECTILE_PROPERTIES: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(0x14),
    };

    pub const CHARACTER_CONTROLLER: RecordReference = RecordReference {
        module: MODULE,
        record: RecordId(0x15),
    };
}

#[macro_export]
macro_rules! panic_handler {
    () => {
        #[cfg(all(not(test), target_family = "wasm"))]
        #[panic_handler]
        fn panic_handler(info: &core::panic::PanicInfo) -> ! {
            game_wasm::error!("{}", info);
            core::arch::wasm32::unreachable()
        }
    };
}
