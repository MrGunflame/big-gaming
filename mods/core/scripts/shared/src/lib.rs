// TODO: Move this crate back to `no_std`.
// This is currently disabled because the
// build is broken because of
// `error: unwinding panics are not supported without std`
// when building on non-wasm targets.
// #![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

mod actor;
mod assets;
mod controller;
mod health;
mod inventory;
mod movement;
mod physics;
mod player;
mod projectile;
mod weapon;
mod weather;
mod world;

use core::f32::consts::PI;

use alloc::vec;
use alloc::vec::Vec;
use bytemuck::Pod;
use bytemuck::Zeroable;
use components::AMMO;
use components::CAMERA;
use components::EQUIPPABLE;
use components::LOOKING_DIRECTION;
use components::PLAYER_CAMERA;
use game_wasm::components::builtin::Collider;
use game_wasm::components::builtin::ColliderShape;
use game_wasm::components::builtin::Cuboid;
use game_wasm::components::builtin::MeshInstance;
use game_wasm::components::builtin::RigidBody;
use game_wasm::components::builtin::RigidBodyKind;
use game_wasm::components::builtin::Transform;
use game_wasm::components::Component;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::events::on_init;
use game_wasm::hierarchy::Children;
use game_wasm::math::Real;
pub use game_wasm::math::Vec3;

use components::MOVEMENT_SPEED;
use game_wasm::entity::EntityId;
use game_wasm::math::Quat;
use game_wasm::resource::ResourceId;
use game_wasm::system::register_action_handler;
use game_wasm::system::register_event_handler;
use game_wasm::system::register_system;
use game_wasm::world::Entity;

use game_wasm::world::RecordReference;
use health::Health;

#[on_init]
pub fn on_init() {
    register_system(
        game_wasm::system::Query {
            components: vec![
                Transform::ID,
                RigidBody::ID,
                Collider::ID,
                CharacterController::ID,
            ],
        },
        controller::drive_character_controller,
    );
    register_system(
        game_wasm::system::Query {
            components: vec![Transform::ID, ProjectileProperties::ID],
        },
        projectile::drive_projectile,
    );

    register_action_handler(movement::move_forward);
    register_action_handler(movement::move_back);
    register_action_handler(movement::move_left);
    register_action_handler(movement::move_right);
    register_action_handler(movement::update_rotation);
    register_action_handler(movement::jump);

    register_action_handler(weapon::weapon_attack);
    register_action_handler(weapon::weapon_reload);

    register_action_handler(inventory::on_equip);
    register_action_handler(inventory::on_uneqip);

    register_action_handler(player::respawn_player);

    register_event_handler(player::spawn_player);
    register_event_handler(player::update_camera_transform);

    register_event_handler(weapon::gun_equip);
    register_event_handler(weapon::gun_unequip);

    register_event_handler(weapon::translate_equipped_items);
    register_event_handler(world::cell_load);
}

pub fn extract_actor_rotation(rotation: Quat) -> Quat {
    debug_assert!(!rotation.is_nan());

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

#[derive(Copy, Clone, Debug, PartialEq, Encode, Decode)]
pub struct MovementSpeed(pub f32);

impl Component for MovementSpeed {
    const ID: RecordReference = MOVEMENT_SPEED;
}

#[derive(Copy, Clone, Debug, Zeroable, Pod, Encode, Decode)]
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

impl Component for GunProperties {
    const ID: RecordReference = components::GUN_PROPERTIES;
}

#[derive(Copy, Clone, Debug, Zeroable, Pod, Encode, Decode)]
#[repr(C)]
pub struct Projectile {
    /// The object id of the projectile that is being fired.
    pub id: RecordReference,
    // FIXME: This is an array so we don't have to bother with
    // alignment.
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
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

impl Component for Ammo {
    const ID: RecordReference = AMMO;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ProjectileProperties {
    pub damage: f32,
    /// Projectile speed in m/s.
    pub speed: f32,
    pub owner: EntityId,
}

impl Component for ProjectileProperties {
    const ID: RecordReference = components::PROJECTILE_PROPERTIES;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SpawnPoint {
    pub translation: Vec3,
}

impl Component for SpawnPoint {
    const ID: RecordReference = components::SPAWN_POINT;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Humanoid;

impl Component for Humanoid {
    const ID: RecordReference = components::HUMANOID;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CharacterController;

impl Component for CharacterController {
    const ID: RecordReference = components::CHARACTER_CONTROLLER;
}

pub mod components {
    use game_wasm::record::{ModuleId, RecordId};
    use game_wasm::world::RecordReference;

    const MODULE: ModuleId = ModuleId::from_str_const("c626b9b0ab1940aba6932ea7726d0175");

    macro_rules! define_id {
        ($($id:ident => $record:expr),*$(,)?) => {
            $(
                pub const $id: RecordReference = RecordReference {
                    module: MODULE,
                    record: RecordId($record),
                };
            )*
        };
    }

    define_id! {
        HUMANOID => 0x06,
        MOVEMENT_SPEED => 0x05,
        GUN_PROPERTIES => 0x0b,
        AMMO => 0x0c,
        HEALTH => 0x13,
        PROJECTILE_PROPERTIES => 0x14,
        SPAWN_POINT => 0x16,
        CHARACTER_CONTROLLER => 0x15,
        EQUIPPABLE => 0x20,
        CAMERA => 0x21,
        PLAYER_CAMERA => 0x22,

        MOVE_FORWARD => 0x01,
        MOVE_BACK => 0x02,
        MOVE_LEFT => 0x03,
        MOVE_RIGHT => 0x04,
        ROTATE => 0x23,

        WEAPON_ATTACK => 0x0d,
        WEAPON_RELOAD => 0x0e,

        EQUIP => 0x17,
        UNEQUIP => 0x18,
        DROP => 0x19,
        PLAYER_RESPAWN => 0x1a,

        TEST_WEAPON => 0x11,

        EQUIPPED_ITEM => 0x20,
        LOOKING_DIRECTION => 0x21,

        // EVENTS
        EVENT_GUN_EQUIP => 0x01,
        EVENT_GUN_UNEQUIP => 0x02,
        TRANSFORM_CHANGED => 0x03,

        SKY_LIGHT => 0x24,

        JUMP => 0x25,

    }
}

fn spawn_player(transform: Transform) -> Entity {
    let entity = Entity::spawn();
    entity.insert(transform);
    entity.insert(MeshInstance {
        model: ResourceId::from(assets::RESOURCE_PERSON),
    });
    entity.insert(RigidBody {
        kind: RigidBodyKind::Fixed,
        linvel: Vec3::ZERO,
        angvel: Vec3::ZERO,
    });
    entity.insert(Collider {
        friction: 1.0,
        restitution: 1.0,
        shape: ColliderShape::Cuboid(Cuboid {
            hx: 1.0,
            hy: 1.0,
            hz: 1.0,
        }),
    });
    entity.insert(MovementSpeed(1.0));
    entity.insert(Humanoid);
    entity.insert(CharacterController);
    entity.insert(Health {
        value: 100,
        max: 100,
    });

    entity
}

#[derive(Copy, Clone, Debug, Zeroable, Pod, Encode, Decode)]
#[repr(C)]
pub struct Equippable {
    pub on_equip: RecordReference,
    pub on_uneqip: RecordReference,
}

impl Component for Equippable {
    const ID: RecordReference = EQUIPPABLE;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Camera {
    pub parent: EntityId,
}

impl Component for Camera {
    const ID: RecordReference = CAMERA;
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerCamera {
    pub camera: EntityId,
    pub offset: Vec3,
    pub rotation: Quat,
}

impl Component for PlayerCamera {
    const ID: RecordReference = PLAYER_CAMERA;
}

#[derive(Copy, Clone, Debug, Default, Encode, Decode)]
pub struct LookingDirection {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl LookingDirection {}

impl Component for LookingDirection {
    const ID: RecordReference = LOOKING_DIRECTION;
}

fn collect_children_recursive(entity: EntityId) -> Vec<EntityId> {
    let mut buf = Vec::new();
    let mut entities = vec![entity];

    for entity in entities.pop() {
        let Ok(children) = Entity::new(entity).get::<Children>() else {
            continue;
        };

        buf.extend(children.get());
        entities.extend(children.get());
    }

    buf
}
