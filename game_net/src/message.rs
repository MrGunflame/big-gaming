use game_common::components::actions::ActionId;
use game_common::components::components::{Components, RawComponent};
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
    InventoryItemUpdate(InventoryItemUpdate),
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

#[derive(Clone, Debug)]
pub struct EntityAction {
    pub entity: ServerEntity,
    pub action: ActionId,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct EntityComponentAdd {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
    pub component: RawComponent,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityComponentRemove {
    pub entity: ServerEntity,
    pub component: RecordReference,
}

#[derive(Clone, Debug)]
pub struct EntityComponentUpdate {
    pub entity: ServerEntity,
    pub component_id: RecordReference,
    pub component: RawComponent,
}

#[derive(Clone, Debug)]
pub struct InventoryItemAdd {
    pub entity: ServerEntity,
    pub id: InventorySlotId,
    pub item: ItemId,
    pub quantity: u32,
    pub components: Components,
    pub equipped: bool,
    pub hidden: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct InventoryItemRemove {
    pub entity: ServerEntity,
    pub slot: InventorySlotId,
}

#[derive(Clone, Debug)]
pub struct InventoryItemUpdate {
    pub entity: ServerEntity,
    pub slot: InventorySlotId,
    pub quantity: Option<u32>,
    pub hidden: bool,
    pub equipped: bool,
    pub components: Option<Components>,
}

impl DataMessageBody {
    pub(crate) fn into_frame(self) -> Frame {
        match self {
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
                bytes: msg.bytes,
            }),
            DataMessageBody::EntityComponentAdd(msg) => {
                Frame::EntityComponentAdd(proto::components::ComponentAdd {
                    entity: msg.entity,
                    component_id: msg.component_id,
                    component: msg.component,
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
                    component_id: msg.component_id,
                    component: msg.component,
                })
            }
            DataMessageBody::InventoryItemAdd(msg) => {
                Frame::InventoryItemAdd(proto::InventoryItemAdd {
                    entity: msg.entity,
                    id: msg.id,
                    item: msg.item,
                    quantity: msg.quantity,
                    components: msg.components,
                    equipped: msg.equipped,
                    hidden: msg.hidden,
                })
            }
            DataMessageBody::InventoryItemRemove(msg) => {
                Frame::InventoryItemRemove(proto::InventoryItemRemove {
                    entity: msg.entity,
                    id: msg.slot,
                })
            }
            DataMessageBody::InventoryItemUpdate(msg) => {
                Frame::InventoryItemUpdate(proto::InventoryItemUpdate {
                    entity: msg.entity,
                    id: msg.slot,
                    equipped: msg.equipped,
                    hidden: msg.hidden,
                    quantity: msg.quantity,
                    components: msg.components,
                })
            }
        }
    }

    pub(crate) fn from_frame(frame: Frame) -> Self {
        match frame {
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
                bytes: frame.bytes,
            }),
            Frame::EntityComponentAdd(frame) => Self::EntityComponentAdd(EntityComponentAdd {
                entity: frame.entity,
                component_id: frame.component_id,
                component: frame.component,
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
                    component_id: frame.component_id,
                    component: frame.component,
                })
            }
            Frame::InventoryItemAdd(frame) => Self::InventoryItemAdd(InventoryItemAdd {
                entity: frame.entity,
                id: frame.id,
                item: frame.item,
                quantity: frame.quantity,
                components: frame.components,
                equipped: frame.equipped,
                hidden: frame.hidden,
            }),
            Frame::InventoryItemRemove(frame) => Self::InventoryItemRemove(InventoryItemRemove {
                entity: frame.entity,
                slot: frame.id,
            }),
            Frame::InventoryItemUpdate(frame) => Self::InventoryItemUpdate(InventoryItemUpdate {
                entity: frame.entity,
                slot: frame.id,
                equipped: frame.equipped,
                quantity: frame.quantity,
                components: frame.components,
                hidden: frame.hidden,
            }),
        }
    }
}
