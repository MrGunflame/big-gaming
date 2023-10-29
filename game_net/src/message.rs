use game_common::components::actions::ActionId;
use game_common::components::inventory::InventorySlotId;
use game_common::components::items::ItemId;
use game_common::net::ServerEntity;
use game_common::record::RecordReference;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::EntityBody;
use glam::{Quat, Vec3};

use crate::proto::{self, Frame};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MessageId(pub u32);

#[derive(Clone, Debug)]
pub enum Message {
    Control(ControlMessage),
    Data(DataMessage),
}

#[derive(Clone, Debug)]
pub enum ControlMessage {
    Connected(),
    Disconnected,
    /// The message was acknowledged by the peer in the given [`ControlFrame`].
    ///
    /// This means that the message was processed at [`ControlFrame`].
    Acknowledge(MessageId, ControlFrame),
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
    EntityComponentAdd(EntityComponentAdd),
    EntityComponentRemove(EntityComponentRemove),
    EntityComponentUpdate(EntityComponentUpdate),
    SpawnHost(SpawnHost),
    InventoryItemAdd(InventoryItemAdd),
    InventoryItemRemove(InventoryItemRemove),
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

#[derive(Clone, Debug)]
pub struct EntityComponentAdd {
    pub entity: ServerEntity,
    pub component: RecordReference,
    pub bytes: Vec<u8>,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityComponentRemove {
    pub entity: ServerEntity,
    pub component: RecordReference,
}

#[derive(Clone, Debug)]
pub struct EntityComponentUpdate {
    pub entity: ServerEntity,
    pub component: RecordReference,
    pub bytes: Vec<u8>,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryItemAdd {
    pub entity: ServerEntity,
    pub id: InventorySlotId,
    pub item: ItemId,
    pub quantity: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryItemRemove {
    pub entity: ServerEntity,
    pub slot: InventorySlotId,
}

impl DataMessageBody {
    pub(crate) fn into_frame(self) -> Frame {
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
            DataMessageBody::EntityComponentAdd(msg) => {
                Frame::EntityComponentAdd(proto::components::ComponentAdd {
                    entity: msg.entity,
                    component_id: msg.component,
                    bytes: msg.bytes,
                })
            }
            DataMessageBody::EntityComponentRemove(msg) => {
                Frame::EntityComponentRemove(proto::components::ComponentRemove {
                    entity: msg.entity,
                    component_id: msg.component,
                })
            }
            DataMessageBody::EntityComponentUpdate(msg) => {
                Frame::EntityComponentUpdate(proto::components::ComponentUpdate {
                    entity: msg.entity,
                    component_id: msg.component,
                    bytes: msg.bytes,
                })
            }
            DataMessageBody::InventoryItemAdd(msg) => {
                Frame::InventoryItemAdd(proto::InventoryItemAdd {
                    entity: msg.entity,
                    id: msg.id,
                    item: msg.item,
                    quantity: msg.quantity,
                })
            }
            DataMessageBody::InventoryItemRemove(msg) => {
                Frame::InventoryItemRemove(proto::InventoryItemRemove {
                    entity: msg.entity,
                    id: msg.slot,
                })
            }
        }
    }

    pub(crate) fn from_frame(frame: Frame) -> Self {
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
            Frame::EntityComponentAdd(frame) => Self::EntityComponentAdd(EntityComponentAdd {
                entity: frame.entity,
                component: frame.component_id,
                bytes: frame.bytes,
            }),
            Frame::EntityComponentRemove(frame) => {
                Self::EntityComponentRemove(EntityComponentRemove {
                    entity: frame.entity,
                    component: frame.component_id,
                })
            }
            Frame::EntityComponentUpdate(frame) => {
                Self::EntityComponentUpdate(EntityComponentUpdate {
                    entity: frame.entity,
                    component: frame.component_id,
                    bytes: frame.bytes,
                })
            }
            Frame::InventoryItemAdd(frame) => Self::InventoryItemAdd(InventoryItemAdd {
                entity: frame.entity,
                id: frame.id,
                item: frame.item,
                quantity: frame.quantity,
            }),
            Frame::InventoryItemRemove(frame) => Self::InventoryItemRemove(InventoryItemRemove {
                entity: frame.entity,
                slot: frame.id,
            }),
            _ => todo!(),
        }
    }
}
