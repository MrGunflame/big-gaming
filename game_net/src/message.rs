use game_common::net::ServerEntity;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::EntityBody;
use glam::{Quat, Vec3};

use crate::proto::{self, Frame};

#[derive(Clone, Debug)]
pub enum Message {
    Control(ControlMessage),
    Data(DataMessage),
}

#[derive(Clone, Debug)]
pub enum ControlMessage {
    Connected(),
    Disconnected,
}

#[derive(Clone, Debug)]
pub struct DataMessage {
    pub control_frame: ControlFrame,
    pub body: DataMessageBody,
}

#[derive(Clone, Debug)]
pub enum DataMessageBody {
    EntityCreate(EntityCreate),
    EntityDestroy(EntityDestroy),
}

#[derive(Clone, Debug)]
pub struct EntityCreate {
    pub entity: ServerEntity,
    pub translation: Vec3,
    pub rotation: Quat,
    pub data: EntityBody,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityDestroy {
    pub entity: ServerEntity,
}

impl DataMessage {
    pub(crate) fn to_frame(self) -> Frame {
        match self.body {
            DataMessageBody::EntityCreate(msg) => Frame::EntityCreate(proto::EntityCreate {
                entity: msg.entity,
                translation: msg.translation,
                rotation: msg.rotation,
                data: msg.data,
            }),
            DataMessageBody::EntityDestroy(msg) => {
                Frame::EntityDestroy(proto::EntityDestroy { entity: msg.entity })
            }
        }
    }

    pub(crate) fn try_from_frame(frame: Frame, cf: ControlFrame) -> Self {
        match frame {
            Frame::EntityCreate(frame) => Self {
                control_frame: cf,
                body: DataMessageBody::EntityCreate(EntityCreate {
                    entity: frame.entity,
                    translation: frame.translation,
                    rotation: frame.rotation,
                    data: frame.data,
                }),
            },
            Frame::EntityDestroy(frame) => Self {
                control_frame: cf,
                body: DataMessageBody::EntityDestroy(EntityDestroy {
                    entity: frame.entity,
                }),
            },
            _ => todo!(),
        }
    }
}
