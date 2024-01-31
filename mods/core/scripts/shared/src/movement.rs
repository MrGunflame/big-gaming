use game_wasm::action::Action;
use game_wasm::components::builtin::{Collider, Transform};
use game_wasm::encoding::{Decode, Encode};
use game_wasm::entity::EntityId;
use game_wasm::events::dispatch_event;
use game_wasm::math::Vec3;
use game_wasm::world::{Entity, RecordReference};
use game_wasm::DT;

use crate::components::{MOVE_BACK, MOVE_FORWARD, MOVE_LEFT, MOVE_RIGHT};
use crate::player::TransformChanged;
use crate::{controller, extract_actor_rotation, Camera, MovementSpeed};

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
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    move_direction(camera.parent, -Vec3::Z);
}

pub fn move_back(entity: EntityId, MoveBack: MoveBack) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    move_direction(camera.parent, Vec3::Z);
}

pub fn move_left(entity: EntityId, MoveLeft: MoveLeft) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    move_direction(camera.parent, -Vec3::X);
}

pub fn move_right(entity: EntityId, MoveRight: MoveRight) {
    let Ok(camera) = Entity::new(entity).get::<Camera>() else {
        return;
    };

    move_direction(camera.parent, Vec3::X);
}

fn move_direction(entity: EntityId, dir: Vec3) {
    let entity = Entity::new(entity);

    let speed = entity.get::<MovementSpeed>().unwrap();
    let mut transform = entity.get::<Transform>().unwrap();
    let collider = entity.get::<Collider>().unwrap();

    let rotation = extract_actor_rotation(transform.rotation);

    let direction = rotation * dir * speed.0 * DT;

    controller::move_shape(entity.id(), &mut transform, direction, &collider.shape);

    entity.insert(transform);

    dispatch_event(&TransformChanged {
        entity: entity.id(),
    });
}
