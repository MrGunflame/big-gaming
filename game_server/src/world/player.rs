use std::f32::consts::PI;

use game_common::components::components::{Component, Components};
use game_common::components::inventory::Inventory;
use game_common::components::items::{Item, ItemId, ItemStack};
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::math::RotationExt;
use game_common::units::Mass;
use game_common::world::world::WorldViewMut;
use game_core::entity::SpawnEntity;
use game_core::modules::Modules;
use glam::{Quat, Vec3};

pub fn spawn_player(modules: &Modules, view: &mut WorldViewMut<'_>) -> SpawnPlayer {
    let race = "c626b9b0ab1940aba6932ea7726d0175:06".parse().unwrap();

    let transform = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));

    let id = SpawnEntity {
        id: race,
        transform,
        is_host: false,
    }
    .spawn(modules, view)
    .unwrap();

    let mut components = Components::new();
    components.insert(
        "c626b9b0ab1940aba6932ea7726d0175:0b".parse().unwrap(),
        Component {
            bytes: vec![
                0, 0, 128, 63, 0, 0, 128, 63, 30, 0, 0, 0, 198, 38, 185, 176, 171, 25, 64, 171,
                166, 147, 46, 167, 114, 109, 1, 117, 18, 0, 0, 0,
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

    // view.inventories_mut()
    //     .get_mut_or_insert(id)
    //     .insert(Item {
    //         id: ItemId("ec7d043851c74c41a35de44befde13b5:06".parse().unwrap()),
    //         mass: Mass::default(),
    //         components: Components::default(),
    //         equipped: false,
    //         hidden: false,
    //     })
    //     .unwrap();

    SpawnPlayer {
        id,
        transform,
        inventory,
    }
}

#[derive(Clone, Debug)]
pub struct SpawnPlayer {
    pub id: EntityId,
    pub transform: Transform,
    pub inventory: Inventory,
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
    let mut pt = rotation.dir_vec();

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
