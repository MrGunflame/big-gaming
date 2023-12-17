use std::f32::consts::PI;

use game_common::components::components::{Component, Components};
use game_common::components::inventory::Inventory;
use game_common::components::items::{Item, ItemId, ItemStack};
use game_common::components::object::ObjectId;
use game_common::components::race::RaceId;
use game_common::components::rendering::MeshInstance;
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::record::RecordReference;
use game_common::world::entity::{Actor, Entity, EntityBody, Object};
use game_core::modules::Modules;
use game_data::record::RecordBody;
use glam::{Quat, Vec3};

use crate::SceneState;

use super::entity::spawn_entity;
use super::state::WorldState;

pub fn spawn_player(
    modules: &Modules,
    world: &mut WorldState,
    state: &mut SceneState,
) -> Option<EntityId> {
    let race_id: RecordReference = "c626b9b0ab1940aba6932ea7726d0175:06".parse().unwrap();

    let transform = Transform::from_translation(Vec3::new(0.0, 40.0, 0.0));

    let Some(module) = modules.get(race_id.module) else {
        return None;
    };

    let Some(record) = module.records.get(race_id.record) else {
        return None;
    };

    let body = match &record.body {
        RecordBody::Item(item) => todo!(),
        RecordBody::Action(_) => return None,
        RecordBody::Component(_) => return None,
        RecordBody::Object(object) => EntityBody::Object(Object {
            id: ObjectId(race_id),
        }),
        RecordBody::Race(race) => EntityBody::Actor(Actor {
            race: RaceId(race_id),
        }),
    };

    let mut components = Components::new();
    for component in &record.components {
        components.insert(
            component.id,
            Component {
                bytes: component.bytes.clone(),
            },
        );
    }

    let entity = Entity {
        id: EntityId::dangling(),
        transform,
        body,
        is_host: false,
        components,
        angvel: Vec3::ZERO,
        linvel: Vec3::ZERO,
    };

    let id = world.spawn();
    world.insert(id, transform);
    world.insert(
        id,
        MeshInstance {
            path: "assets/box.glb".to_owned(),
        },
    );

    let mut components = Components::new();
    components.insert(
        "c626b9b0ab1940aba6932ea7726d0175:0b".parse().unwrap(),
        Component {
            bytes: vec![
                0, 0, 128, 63, 0, 0, 128, 63, 30, 0, 0, 0, 198, 38, 185, 176, 171, 25, 64, 171,
                166, 147, 46, 167, 114, 109, 1, 117, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63,
            ],
        },
    );

    let mut inventory = Inventory::new();
    inventory
        .insert(ItemStack {
            item: Item {
                id: ItemId("c626b9b0ab1940aba6932ea7726d0175:11".parse().unwrap()),
                mass: Default::default(),
                components,
                equipped: true,
                hidden: false,
            },
            quantity: 1,
        })
        .unwrap();

    world.insert_inventory(id, inventory);

    // let key = spawn_entity(entity, world, state, modules);
    // state.entities.insert(key, id);

    Some(id)
}

// pub fn move_player(event: PlayerMove, entity_id: EntityId, view: &mut WorldViewMut<'_>) {
//     let Some(mut entity) = view.get_mut(entity_id) else {
//         return;
//     };

//     let speed = 1.0;

//     // FIXME: This is not quite correct, if the entity moves along two axes it
//     // should not move along both with the speed as if it were moving into one
//     // direction. (i.e. Forward+Left moves the player less along both the Forward
//     // and left axes than just a Foward/Left command).
//     let dir = (event.bits.forward as u8 as f32) * -Vec3::Z
//         + (event.bits.back as u8 as f32) * Vec3::Z
//         + (event.bits.left as u8 as f32) * -Vec3::X
//         + (event.bits.right as u8 as f32) * Vec3::X;

//     let delta = extract_actor_rotation(entity.transform.rotation) * dir * speed;
//     dbg!(delta);
//     entity.set_translation(entity.transform.translation + delta);

//     dbg!(entity.transform.translation);
// }

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
