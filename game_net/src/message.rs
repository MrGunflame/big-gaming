use game_common::components::actions::ActionId;
use game_common::net::ServerEntity;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::EntityBody;
use glam::{Quat, Vec3};

use crate::proto::{self, Frame};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MessageId(pub(crate) u32);

#[derive(Clone, Debug)]
pub enum Message {
    Control(ControlMessage),
    Data(DataMessage),
}

#[derive(Clone, Debug)]
pub enum ControlMessage {
    Connected(),
    Disconnected,
    Acknowledge(MessageId),
}

#[derive(Clone, Debug)]
pub struct DataMessage {
    pub id: MessageId,
    pub control_frame: ControlFrame,
    pub body: DataMessageBody,
}

#[derive(Clone, Debug)]
pub enum DataMessageBody {
    EntityCreate(EntityCreate),
    EntityDestroy(EntityDestroy),
    EntityTranslate(EntityTranslate),
    EntityRotate(EntityRotate),
    EntityAction(EntityAction),
    SpawnHost(SpawnHost),
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

#[derive(Copy, Clone, Debug)]
pub struct SpawnHost {
    pub entity: ServerEntity,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityTranslate {
    pub entity: ServerEntity,
    pub translation: Vec3,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityRotate {
    pub entity: ServerEntity,
    pub rotation: Quat,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityAction {
    pub entity: ServerEntity,
    pub action: ActionId,
}

impl DataMessageBody {
    pub(crate) fn to_frame(self) -> Frame {
        match self {
            DataMessageBody::EntityCreate(msg) => Frame::EntityCreate(proto::EntityCreate {
                entity: msg.entity,
                translation: msg.translation,
                rotation: msg.rotation,
                data: msg.data,
            }),
            DataMessageBody::EntityDestroy(msg) => {
                Frame::EntityDestroy(proto::EntityDestroy { entity: msg.entity })
            }
            DataMessageBody::SpawnHost(msg) => {
                Frame::SpawnHost(proto::SpawnHost { entity: msg.entity })
            }
            DataMessageBody::EntityTranslate(msg) => {
                Frame::EntityTranslate(proto::EntityTranslate {
                    entity: msg.entity,
                    translation: msg.translation,
                })
            }
            DataMessageBody::EntityRotate(msg) => Frame::EntityRotate(proto::EntityRotate {
                entity: msg.entity,
                rotation: msg.rotation,
            }),
            DataMessageBody::EntityAction(msg) => Frame::EntityAction(proto::EntityAction {
                entity: msg.entity,
                action: msg.action,
            }),
        }
    }

    pub(crate) fn from_frame(frame: Frame, cf: ControlFrame) -> Self {
        match frame {
            Frame::EntityCreate(frame) => Self::EntityCreate(EntityCreate {
                entity: frame.entity,
                translation: frame.translation,
                rotation: frame.rotation,
                data: frame.data,
            }),
            Frame::EntityDestroy(frame) => Self::EntityDestroy(EntityDestroy {
                entity: frame.entity,
            }),
            Frame::SpawnHost(frame) => Self::SpawnHost(SpawnHost {
                entity: frame.entity,
            }),
            Frame::EntityTranslate(frame) => Self::EntityTranslate(EntityTranslate {
                entity: frame.entity,
                translation: frame.translation,
            }),
            Frame::EntityRotate(frame) => Self::EntityRotate(EntityRotate {
                entity: frame.entity,
                rotation: frame.rotation,
            }),
            Frame::EntityAction(frame) => Self::EntityAction(EntityAction {
                entity: frame.entity,
                action: frame.action,
            }),
            _ => todo!(),
        }
    }
}
