mod conn;
mod entities;
mod prediction;
mod socket;
pub mod world;

use std::collections::VecDeque;

use game_common::components::components::{Component, Components};
use game_common::components::items::{Item, ItemStack};
use game_common::components::transform::Transform;
use game_common::units::Mass;
use game_common::world::entity::EntityBody;
use game_core::entity::SpawnEntity;
use game_net::message::{ControlMessage, DataMessageBody, Message};
use glam::Vec3;

use crate::entities::terrain::spawn_terrain;

pub use self::conn::ServerConnection;

fn flush_command_queue<I>(conn: &mut ServerConnection<I>) {
    let mut queue = VecDeque::new();
    let handle = conn.handle.as_ref().unwrap();
    while let Some(msg) = handle.recv() {
        queue.push_back(msg);
    }

    while let Some(msg) = queue.pop_front() {
        let msg = match msg {
            Message::Control(ControlMessage::Connected()) => {
                continue;
            }
            Message::Control(ControlMessage::Disconnected) => {
                conn.shutdown();
                continue;
            }
            Message::Control(ControlMessage::Acknowledge(id, cf)) => {
                conn.input_buffer.remove(cf, id);
                continue;
            }
            Message::Data(msg) => msg,
        };

        if conn.world.get(msg.control_frame).is_none() {
            conn.world.insert(msg.control_frame);
        }

        let mut view = conn.world.get_mut(msg.control_frame).unwrap();

        // let Some(mut view) = conn.world.get_mut(msg.control_frame) else {
        //     // If the control frame does not exist on the client ast least one of these issues are to blame:
        //     // 1. The server is sending garbage data, refereing to a control frame that has either already
        //     //    passed or is still too far in the future.
        //     // 2. The client's clock is desynced and creating new snapshots too slow/fast.
        //     // 3. The server's clock is desynced and creating new snapshots too slow/fast.
        //     // let front = conn.world.front().unwrap();
        //     // let back = conn.world.back().unwrap();
        //     // tracing::warn!(
        //     //     "received snapshot for unknwon control frame: {:?} (snapshots  {:?}..={:?} exist)",
        //     //     msg.control_frame,
        //     //     front.control_frame(),
        //     //     back.control_frame()
        //     // );
        //     // continue;

        //     conn.world.insert(msg.control_frame);
        //     conn.world.get_mut(msg.control_frame).unwrap()
        // };

        match msg.body {
            DataMessageBody::EntityCreate(msg) => {
                let id = match msg.data {
                    EntityBody::Actor(actor) => actor.race.0,
                    EntityBody::Object(object) => object.id.0,
                    EntityBody::Item(item) => item.id.0,
                    EntityBody::Terrain(terrain) => todo!(),
                };

                let id = SpawnEntity {
                    id,
                    transform: Transform {
                        translation: msg.translation,
                        rotation: msg.rotation,
                        scale: Vec3::splat(1.0),
                    },
                    is_host: false,
                }
                .spawn(&conn.modules, &mut view)
                .unwrap();

                conn.server_entities.insert(id, msg.entity);
            }
            DataMessageBody::EntityDestroy(msg) => match conn.server_entities.remove(msg.entity) {
                Some(id) => {
                    if view.despawn(id).is_none() {
                        tracing::warn!("attempted to destroy a non-existant entity {:?}", id);
                    }
                }
                None => (),
            },
            DataMessageBody::EntityTranslate(msg) => match conn.server_entities.get(msg.entity) {
                Some(id) => match view.get_mut(id) {
                    Some(mut entity) => {
                        entity.set_translation(msg.translation);
                    }
                    None => {
                        tracing::warn!("received translation for unknown entity {:?}", id);
                    }
                },
                None => (),
            },
            DataMessageBody::EntityRotate(msg) => match conn.server_entities.get(msg.entity) {
                Some(id) => match view.get_mut(id) {
                    Some(mut entity) => entity.set_rotation(msg.rotation),
                    None => {
                        tracing::warn!("received rotation for unknown entity {:?}", id);
                    }
                },
                None => (),
            },
            DataMessageBody::SpawnHost(msg) => match conn.server_entities.get(msg.entity) {
                Some(id) => {
                    view.spawn_host(id);
                }
                None => (),
            },
            DataMessageBody::EntityAction(msg) => todo!(),
            DataMessageBody::InventoryItemAdd(msg) => {
                let stack = ItemStack {
                    item: Item {
                        id: msg.item,
                        mass: Mass::default(),
                        components: Components::default(),
                        equipped: false,
                        hidden: false,
                    },
                    quantity: msg.quantity,
                };

                match conn.server_entities.get(msg.entity) {
                    Some(id) => {
                        view.inventory_insert_items(id, msg.id, stack);
                    }
                    None => (),
                }
            }
            DataMessageBody::EntityComponentAdd(msg) => {
                match conn.server_entities.get(msg.entity) {
                    Some(id) => {
                        let mut entity = view.get_mut(id).unwrap();
                        entity.insert_component(msg.component, Component { bytes: msg.bytes });
                    }
                    None => (),
                }
            }
            DataMessageBody::EntityComponentRemove(msg) => {
                match conn.server_entities.get(msg.entity) {
                    Some(id) => {
                        let mut entity = view.get_mut(id).unwrap();
                        entity.remove_component(msg.component);
                    }
                    None => (),
                }
            }
            DataMessageBody::EntityComponentUpdate(msg) => {
                match conn.server_entities.get(msg.entity) {
                    Some(id) => {
                        let mut entity = view.get_mut(id).unwrap();
                        entity.update_component(msg.component, Component { bytes: msg.bytes });
                    }
                    None => (),
                }
            }
        }
    }
}
