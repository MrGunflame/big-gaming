use game_wasm::action::Action;
use game_wasm::components::builtin::{Collider, Transform};
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::math::Vec3;
use game_wasm::world::{Entity, RecordReference};

use crate::components::{MOVE_BACK, MOVE_FORWARD, MOVE_LEFT, MOVE_RIGHT};
use crate::{controller, extract_actor_rotation, MovementSpeed, UPS};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct MoveForward;

impl Action for MoveForward {
    const ID: RecordReference = MOVE_FORWARD;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct MoveBack;

impl Action for MoveBack {
    const ID: RecordReference = MOVE_BACK;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct MoveLeft;

impl Action for MoveLeft {
    const ID: RecordReference = MOVE_LEFT;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct MoveRight;

impl Action for MoveRight {
    const ID: RecordReference = MOVE_RIGHT;
}

pub fn move_forward(entity: EntityId, MoveForward: MoveForward) {
    move_direction(entity, -Vec3::Z);
}

pub fn move_back(entity: EntityId, MoveBack: MoveBack) {
    move_direction(entity, Vec3::Z);
}

pub fn move_left(entity: EntityId, MoveLeft: MoveLeft) {
    move_direction(entity, -Vec3::X);
}

pub fn move_right(entity: EntityId, MoveRight: MoveRight) {
    move_direction(entity, Vec3::X);
}

fn move_direction(entity: EntityId, dir: Vec3) {
    let entity = Entity::new(entity);

    let speed = entity.get::<MovementSpeed>().unwrap();
    let mut transform = entity.get::<Transform>().unwrap();
    let collider = entity.get::<Collider>().unwrap();

    let rotation = extract_actor_rotation(transform.rotation);

    let direction = rotation * dir * (speed.0 / UPS);

    controller::move_shape(entity.id(), &mut transform, direction, &collider.shape);

    entity.insert(transform);
}
