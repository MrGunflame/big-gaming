mod conn;
mod entities;
mod prediction;
mod socket;
pub mod world;

use std::collections::VecDeque;

use game_common::components::transform::Transform;
use game_common::world::entity::EntityBody;
use game_core::entity::SpawnEntity;
use game_net::message::{ControlMessage, DataMessageBody, Message};
use glam::Vec3;

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
                // FIXME: Somehow the server tends to run 1 CF ahead of the client.
                // Reducing the CF by 1 magically resolves all problems for low latency
                // connections, but is this actually correct?
                conn.input_buffer.remove(cf - 1, id);
                continue;
            }
            Message::Data(msg) => msg,
        };

        let Some(mut view) = conn.world.get_mut(msg.control_frame) else {
            // If the control frame does not exist on the client ast least one of these issues are to blame:
            // 1. The server is sending garbage data, refereing to a control frame that has either already
            //    passed or is still too far in the future.
            // 2. The client's clock is desynced and creating new snapshots too slow/fast.
            // 3. The server's clock is desynced and creating new snapshots too slow/fast.
            let front = conn.world.front().unwrap();
            let back = conn.world.back().unwrap();
            tracing::warn!(
                "received snapshot for unknwon control frame: {:?} (snapshots  {:?}..={:?} exist)",
                msg.control_frame,
                front.control_frame(),
                back.control_frame()
            );
            continue;
        };

        match msg.body {
            DataMessageBody::EntityCreate(msg) => match msg.data {
                EntityBody::Actor(actor) => {
                    let id = SpawnEntity {
                        id: actor.race.0,
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
                _ => todo!(),
            },
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
        }
    }
}
