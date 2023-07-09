//! Entity translations

use crate::proto::{
    EntityAction, EntityCreate, EntityDestroy, EntityHealth, EntityRotate, EntityTranslate, Frame,
    InventoryItemAdd, InventoryItemRemove, InventoryItemUpdate, SpawnHost,
};
use crate::snapshot::Command;

use crate::snapshot::{
    EntityAction as EntityActionCommand, EntityCreate as EntityCreateCommand,
    EntityDestroy as EntityDestroyCommand, EntityHealth as EntityHealthCommand,
    EntityRotate as EntityRotateCommand, EntityTranslate as EntityTranslateCommand,
    InventoryItemAdd as InventoryItemAddCommand, InventoryItemRemove as InventoryItemRemoveCommand,
    InventoryUpdate as InventoryUpdateCommand, SpawnHost as SpawnHostCommand,
};

pub(crate) fn unpack_command(frame: Frame) -> Option<Command> {
    match frame {
        Frame::EntityCreate(frame) => Some(Command::EntityCreate(EntityCreateCommand {
            id: frame.entity,
            translation: frame.translation,
            rotation: frame.rotation,
            data: frame.data,
        })),
        Frame::EntityDestroy(frame) => Some(Command::EntityDestroy(EntityDestroyCommand {
            id: frame.entity,
        })),
        Frame::EntityTranslate(frame) => Some(Command::EntityTranslate(EntityTranslateCommand {
            id: frame.entity,
            translation: frame.translation,
        })),
        Frame::EntityRotate(frame) => Some(Command::EntityRotate(EntityRotateCommand {
            id: frame.entity,
            rotation: frame.rotation,
        })),
        Frame::EntityHealth(frame) => Some(Command::EntityHealth(EntityHealthCommand {
            id: frame.entity,
            health: frame.health,
        })),
        Frame::EntityAction(frame) => Some(Command::EntityAction(EntityActionCommand {
            id: frame.entity,
            action: frame.action,
        })),
        Frame::SpawnHost(frame) => Some(Command::SpawnHost(SpawnHostCommand { id: frame.entity })),
        Frame::InventoryItemAdd(frame) => {
            Some(Command::InventoryItemAdd(InventoryItemAddCommand {
                entity: frame.entity,
                slot: frame.id,
                item: frame.item,
            }))
        }
        Frame::InventoryItemRemove(frame) => {
            Some(Command::InventoryItemRemove(InventoryItemRemoveCommand {
                entity: frame.entity,
                slot: frame.id,
            }))
        }
        Frame::InventoryItemUpdate(frame) => {
            Some(Command::InventoryUpdate(InventoryUpdateCommand {
                entity: frame.entity,
                slot: frame.id,
                equipped: frame.equipped,
                hidden: frame.hidden,
            }))
        }
    }
}

pub(crate) fn pack_command(cmd: &Command) -> Option<Frame> {
    match cmd {
        Command::EntityCreate(cmd) => Some(Frame::EntityCreate(EntityCreate {
            entity: cmd.id,
            translation: cmd.translation,
            rotation: cmd.rotation,
            data: cmd.data.clone(),
        })),
        Command::EntityDestroy(cmd) => Some(Frame::EntityDestroy(EntityDestroy { entity: cmd.id })),
        Command::EntityTranslate(cmd) => Some(Frame::EntityTranslate(EntityTranslate {
            entity: cmd.id,
            translation: cmd.translation,
        })),
        Command::EntityRotate(cmd) => Some(Frame::EntityRotate(EntityRotate {
            entity: cmd.id,
            rotation: cmd.rotation,
        })),
        Command::EntityHealth(cmd) => Some(Frame::EntityHealth(EntityHealth {
            entity: cmd.id,
            health: cmd.health,
        })),
        Command::EntityAction(cmd) => Some(Frame::EntityAction(EntityAction {
            entity: cmd.id,
            action: cmd.action,
        })),
        Command::SpawnHost(cmd) => Some(Frame::SpawnHost(SpawnHost { entity: cmd.id })),
        Command::InventoryItemAdd(cmd) => Some(Frame::InventoryItemAdd(InventoryItemAdd {
            entity: cmd.entity,
            id: cmd.slot,
            item: cmd.item,
        })),
        Command::InventoryItemRemove(cmd) => {
            Some(Frame::InventoryItemRemove(InventoryItemRemove {
                entity: cmd.entity,
                id: cmd.slot,
            }))
        }
        Command::InventoryUpdate(cmd) => Some(Frame::InventoryItemUpdate(InventoryItemUpdate {
            entity: cmd.entity,
            id: cmd.slot,
            equipped: cmd.equipped,
            hidden: cmd.hidden,
        })),
        Command::Connected(_) => None,
        Command::Disconnected => None,
        Command::ReceivedCommands(_) => None,
    }
}
