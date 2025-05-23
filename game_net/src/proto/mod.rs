//! The client/server protcol
//!
//! # Packet vs Frame
//!
//! The protocol distinguishes between [`Packet`]s and [`Frame`]s.
//!
//! A [`Packet`] is the payload that is transmitted over the network link. It is never greater
//! than the MTU (typically 1500). A [`Packet`] always begins with a [`Header`] which contains the
//! packets metadata.
//!
//! A [`Frame`] is a single event that represents a change to the game world. A [`Packet`] may
//! contain multiple events.
//!
//!
//!
//! # Server enitity ids
//!
//! Every entity that needs to be synchronized between the server and client is represented by a
//! [`ServerEntity`] which uniquely identifies an entity for both sides. [`ServerEntity`]s are only
//! created by the server, and accepted by the client.
//!
//! Entities are created by the [`EntityCreate`] frame and destroyed by the [`EntityDestroy`]
//! frame. A [`ServerEntity`] may be reused once the entity has been destroyed, and the client has
//! acknowledged the reception of that [`Packet`].
//!
//! The generation algorithm for [`ServerEntity`] ids is unspecified and the choice is left to the
//! server implementation.
//!

pub mod ack;
pub mod components;
pub mod handshake;
pub mod sequence;
pub mod shutdown;

mod action;
mod quat;
mod record;
mod terrain;
mod varint;

use game_common::components::actions::ActionId;
use game_common::components::object::ObjectId;
use game_common::components::race::RaceId;
use game_common::record::RecordReference;
use game_common::world::control_frame::ControlFrame;
use game_common::world::entity::{Actor, EntityBody, Object, Terrain};
use game_common::world::CellId;
pub use game_macros::{net__decode as Decode, net__encode as Encode};

use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::net::{ServerEntity, ServerResource};
use glam::{Quat, UVec2, Vec3};
use thiserror::Error;

use self::ack::{Ack, AckAck, Nak};
use self::components::{ComponentAdd, ComponentRemove, ComponentUpdate};
use self::handshake::{Handshake, InvalidHandshakeFlags, InvalidHandshakeType};
use self::sequence::Sequence;
use self::shutdown::Shutdown;
use self::varint::VarInt;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error(transparent)]
pub enum Error {
    UnexpectedEof(#[from] EofError),
    InvalidPacketType(#[from] InvalidPacketType),
    InvalidFrameType(#[from] InvalidFrameType),
    InvalidEntityKind(#[from] InvalidEntityKind),
    InvalidHandshakeType(#[from] InvalidHandshakeType),
    InvalidHandshakeFlags(#[from] InvalidHandshakeFlags),
}

impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("unexpected eof: expected {expected} bytes, found {found}")]
pub struct EofError {
    pub expected: usize,
    pub found: usize,
}

pub trait Encode {
    type Error;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut;
}

pub trait Decode: Sized {
    type Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf;
}

macro_rules! impl_primitive {
    ($($t:ty),*$(,)?) => {
        $(
            impl Encode for $t {
                type Error = Infallible;

                fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
                where
                    B: BufMut,
                {
                    buf.put_slice(&self.to_be_bytes());
                    Ok(())
                }
            }

            impl Decode for $t {
                type Error = EofError;

                fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
                where
                    B: Buf
                {
                    if buf.remaining() < std::mem::size_of::<Self>() {
                        return Err(EofError {
                            expected: std::mem::size_of::<Self>(),
                            found: buf.remaining(),
                        });
                    }

                    let mut bytes = [0; std::mem::size_of::<Self>()];
                    buf.copy_to_slice(&mut bytes);
                    Ok(Self::from_be_bytes(bytes))
                }
            }
        )*
    };
}

impl_primitive! { u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64 }

impl Encode for () {
    type Error = Infallible;

    fn encode<B>(&self, _buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        Ok(())
    }
}

impl Decode for () {
    type Error = Infallible;

    fn decode<B>(_buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(())
    }
}

impl Encode for Vec3 {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.x.encode(&mut buf)?;
        self.y.encode(&mut buf)?;
        self.z.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for Vec3 {
    type Error = EofError;

    #[inline]
    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = f32::decode(&mut buf)?;
        let y = f32::decode(&mut buf)?;
        let z = f32::decode(&mut buf)?;
        Ok(Self { x, y, z })
    }
}

impl Encode for UVec2 {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.x.encode(&mut buf)?;
        self.y.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for UVec2 {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = u32::decode(&mut buf)?;
        let y = u32::decode(&mut buf)?;
        Ok(Self::new(x, y))
    }
}

impl Encode for ServerEntity {
    type Error = Infallible;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        VarInt(self.0).encode(buf)
    }
}

impl Decode for ServerEntity {
    type Error = EofError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        VarInt::decode(buf).map(|val| Self(val.0))
    }
}

impl Encode for CellId {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        let (x, y, z) = self.as_parts();
        x.encode(&mut buf)?;
        y.encode(&mut buf)?;
        z.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for CellId {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = u32::decode(&mut buf)?;
        let y = u32::decode(&mut buf)?;
        let z = u32::decode(&mut buf)?;
        Ok(Self::from_parts(x, y, z))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PacketType(u16);

impl PacketType {
    pub const HANDSHAKE: Self = Self(0);
    pub const SHUTDOWN: Self = Self(1);

    pub const ACK: Self = Self(2);
    pub const ACKACK: Self = Self(3);
    pub const NAK: Self = Self(4);

    pub const DATA: Self = Self(5);
}

impl Encode for PacketType {
    type Error = Infallible;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)?;
        Ok(())
    }
}

impl Decode for PacketType {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(Self::try_from(u16::decode(buf)?)?)
    }
}

impl TryFrom<u16> for PacketType {
    type Error = InvalidPacketType;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match Self(value) {
            Self::HANDSHAKE => Ok(Self::HANDSHAKE),
            Self::SHUTDOWN => Ok(Self::SHUTDOWN),
            Self::ACK => Ok(Self::ACK),
            Self::ACKACK => Ok(Self::ACKACK),
            Self::NAK => Ok(Self::NAK),
            Self::DATA => Ok(Self::DATA),
            _ => Err(InvalidPacketType(value)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("invalid packet type: {0}")]
pub struct InvalidPacketType(pub u16);

impl From<u16> for InvalidPacketType {
    #[inline]
    fn from(value: u16) -> Self {
        Self(value)
    }
}

///
/// Data packet:
///
/// ```text
///  0               1               2               3
///  0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |0| Sequence                                                    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Control Frame                   | Flags                       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// Control packet:
///
/// ```text
///  0               1               2               3
///  0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7 0 1 2 3 4 5 6 7
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |1| Control Type                | Reserved                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Reserved                      | Flags                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Header {
    pub packet_type: PacketType,
    pub sequence: Sequence,
    pub control_frame: ControlFrame,
    pub flags: Flags,
}

impl Encode for Header {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        let word0 = if self.packet_type == PacketType::DATA {
            let bits = self.sequence.to_bits();

            // MSB must not be set.
            // This should be enforced by the Sequence type.
            #[cfg(debug_assertions)]
            assert!(bits < 1 << 31);

            bits
        } else {
            let packet_type = 1 << 31;
            let control_type = (self.packet_type.0 as u32) << 16;

            packet_type | control_type
        };

        word0.encode(&mut buf)?;
        self.control_frame.encode(&mut buf)?;
        self.flags.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for Header {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let word0 = u32::decode(&mut buf)?;
        let control_frame = ControlFrame::decode(&mut buf)?;
        let flags = Flags::decode(&mut buf)?;

        let packet_type;
        let sequence;
        // DATA
        if word0 & (1 << 31) == 0 {
            packet_type = PacketType::DATA;
            sequence = Sequence::from_bits(word0 & (u32::MAX >> 1));

            // CONTROL
        } else {
            // Decode the control type.
            let bits = ((word0 >> 16) & ((1 << 15) - 1)) as u16;
            packet_type = PacketType::try_from(bits)?;
            sequence = Sequence::new(0);
        }

        Ok(Self {
            packet_type,
            sequence,
            control_frame,
            flags,
        })
    }
}

/// Flags
///
/// ```text
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |P P|R|                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Flags(u16);

impl Flags {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn retransmission(self) -> bool {
        (self.0 & 0b0010_0000_0000_0000) != 0
    }

    pub fn set_retransmission(&mut self, v: bool) {
        if v {
            self.0 |= 0b0010_0000_0000_0000;
        } else {
            self.0 &= !0b0010_0000_0000_0000
        }
    }

    pub fn packet_position(self) -> PacketPosition {
        match self.0 & 0b1100_0000_0000_0000 {
            0b0000_0000_0000_0000 => PacketPosition::Single,
            0b1000_0000_0000_0000 => PacketPosition::First,
            0b0100_0000_0000_0000 => PacketPosition::Last,
            0b1100_0000_0000_0000 => PacketPosition::Middle,
            _ => unreachable!(),
        }
    }

    pub fn set_packet_position(&mut self, pos: PacketPosition) {
        self.0 &= !0b1100_0000_0000_0000;

        self.0 |= match pos {
            PacketPosition::Single => 0b0000_0000_0000_0000,
            PacketPosition::First => 0b1000_0000_0000_0000,
            PacketPosition::Last => 0b0100_0000_0000_0000,
            PacketPosition::Middle => 0b1100_0000_0000_0000,
        };
    }
}

impl Encode for Flags {
    type Error = <u16 as Encode>::Error;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for Flags {
    type Error = <u16 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u16::decode(buf).map(Self)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PacketPosition {
    #[default]
    Single,
    First,
    Middle,
    Last,
}

impl PacketPosition {
    #[inline]
    pub const fn is_single(self) -> bool {
        matches!(self, Self::Single)
    }

    #[inline]
    pub const fn is_first(self) -> bool {
        matches!(self, Self::First)
    }

    #[inline]
    pub const fn is_middle(self) -> bool {
        matches!(self, Self::Middle)
    }

    #[inline]
    pub const fn is_last(self) -> bool {
        matches!(self, Self::Last)
    }
}

/// Creates a new entity on the client.
#[derive(Clone, Debug, Encode, Decode)]
pub struct EntityCreate {
    pub entity: ServerEntity,
    pub translation: Vec3,
    pub rotation: Quat,
    pub data: EntityBody,
}

impl Encode for EntityBody {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        match self {
            Self::Terrain(terrain) => {
                0u8.encode(&mut buf)?;
                terrain.encode(&mut buf)?;
            }
            Self::Object(object) => {
                1u8.encode(&mut buf)?;
                object.id.encode(&mut buf)?;
            }
            Self::Actor(actor) => {
                2u8.encode(&mut buf)?;
                actor.race.encode(&mut buf)?;
            }
        }

        Ok(())
    }
}

impl Decode for EntityBody {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let typ = u8::decode(&mut buf)?;

        match typ {
            0u8 => {
                let terrain = Terrain::decode(&mut buf)?;
                Ok(Self::Terrain(terrain))
            }
            1u8 => {
                let id = ObjectId::decode(&mut buf)?;

                Ok(Self::Object(Object { id }))
            }
            2u8 => {
                let race = RaceId::decode(&mut buf)?;

                Ok(Self::Actor(Actor { race }))
            }
            _ => Err(InvalidEntityKind(typ).into()),
        }
    }
}

impl Encode for RaceId {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for RaceId {
    type Error = EofError;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        RecordReference::decode(buf).map(Self)
    }
}

// impl Encode for EntityCreate {
//     type Error = Infallible;

//     fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
//     where
//         B: BufMut,
//     {
//         self.entity.encode(&mut buf)?;
//         // TODO: Bit packing
//         let kind: u8 = match self.kind {
//             EntityKind::Object => 1,
//             EntityKind::Actor => 2,
//         };
//         kind.encode(&mut buf)?;
//         self.translation.encode(&mut buf)?;
//         self.rotation.encode(&mut buf)?;

//         Ok(())
//     }
// }

/// Destroys a entity on the client.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityDestroy {
    pub entity: ServerEntity,
}

/// Update the translation of an entity.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityTranslate {
    pub entity: ServerEntity,
    /// The new translation (absolute) translation of the entity.
    pub translation: Vec3,
}

/// Update the rotation of an entity. Contains the new rotation.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityRotate {
    pub entity: ServerEntity,
    /// The new rotation (absolute) of the entity.
    pub rotation: Quat,
}

#[derive(Clone, Debug)]
pub struct EntityAction {
    pub entity: ServerEntity,
    pub action: ActionId,
    pub bytes: Vec<u8>,
}

impl Encode for EntityAction {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.entity.encode(&mut buf)?;
        self.action.encode(&mut buf)?;
        (self.bytes.len() as u64).encode(&mut buf)?;
        buf.put_slice(&self.bytes);
        Ok(())
    }
}

impl Decode for EntityAction {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let entity = ServerEntity::decode(&mut buf)?;
        let action = ActionId::decode(&mut buf)?;
        let len = u64::decode(&mut buf)?;

        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(u8::decode(&mut buf)?);
        }

        Ok(Self {
            entity,
            action,
            bytes,
        })
    }
}

/// Sets the host actor host used by the client.
///
/// This is the actor the client is allowed to control.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SpawnHost {
    pub entity: ServerEntity,
}

#[derive(Clone, Debug)]
pub enum Frame {
    EntityDestroy(EntityDestroy),
    EntityTranslate(EntityTranslate),
    EntityRotate(EntityRotate),
    EntityAction(EntityAction),
    EntityComponentAdd(ComponentAdd),
    EntityComponentRemove(ComponentRemove),
    EntityComponentUpdate(ComponentUpdate),
    SpawnHost(SpawnHost),
    ResourceCreate(ResourceCreate),
    ResourceDestroy(ResourceDestroy),
}

impl Frame {
    pub fn id(&self) -> Option<ServerEntity> {
        match self {
            Self::EntityDestroy(frame) => Some(frame.entity),
            Self::EntityTranslate(frame) => Some(frame.entity),
            Self::EntityRotate(frame) => Some(frame.entity),
            Self::EntityAction(frame) => Some(frame.entity),
            Self::EntityComponentAdd(frame) => Some(frame.entity),
            Self::EntityComponentRemove(frame) => Some(frame.entity),
            Self::EntityComponentUpdate(frame) => Some(frame.entity),
            Self::SpawnHost(frame) => Some(frame.entity),
            Self::ResourceCreate(_) | Self::ResourceDestroy(_) => None,
        }
    }
}

impl Encode for Frame {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        match self {
            Self::EntityDestroy(frame) => {
                FrameType::ENTITY_DESTROY.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityTranslate(frame) => {
                FrameType::ENTITY_TRANSLATE.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityRotate(frame) => {
                FrameType::ENTITY_ROTATE.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityAction(frame) => {
                FrameType::ENTITY_ACTION.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityComponentAdd(frame) => {
                FrameType::ENTITY_COMPONENT_ADD.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityComponentRemove(frame) => {
                FrameType::ENTITY_COMPONENT_REMOVE.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityComponentUpdate(frame) => {
                FrameType::ENTITY_COMPONENT_UPDATE.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::SpawnHost(frame) => {
                FrameType::SPAWN_HOST.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::ResourceCreate(frame) => {
                FrameType::RESOURCE_CREATE.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::ResourceDestroy(frame) => {
                FrameType::RESOURCE_DESTROY.encode(&mut buf)?;
                frame.encode(buf)
            }
        }
    }
}

impl Decode for Frame {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let typ = FrameType::decode(&mut buf)?;

        match typ {
            FrameType::ENTITY_DESTROY => {
                let frame = EntityDestroy::decode(buf)?;
                Ok(Self::EntityDestroy(frame))
            }
            FrameType::ENTITY_TRANSLATE => {
                let frame = EntityTranslate::decode(buf)?;
                Ok(Self::EntityTranslate(frame))
            }
            FrameType::ENTITY_ROTATE => {
                let frame = EntityRotate::decode(buf)?;
                Ok(Self::EntityRotate(frame))
            }
            FrameType::ENTITY_ACTION => {
                let frame = EntityAction::decode(buf)?;
                Ok(Self::EntityAction(frame))
            }
            FrameType::ENTITY_COMPONENT_ADD => {
                let frame = ComponentAdd::decode(buf)?;
                Ok(Self::EntityComponentAdd(frame))
            }
            FrameType::ENTITY_COMPONENT_REMOVE => {
                let frame = ComponentRemove::decode(buf)?;
                Ok(Self::EntityComponentRemove(frame))
            }
            FrameType::ENTITY_COMPONENT_UPDATE => {
                let frame = ComponentUpdate::decode(buf)?;
                Ok(Self::EntityComponentUpdate(frame))
            }
            FrameType::SPAWN_HOST => {
                let frame = SpawnHost::decode(buf)?;
                Ok(Self::SpawnHost(frame))
            }
            FrameType::RESOURCE_CREATE => {
                let frame = ResourceCreate::decode(buf)?;
                Ok(Self::ResourceCreate(frame))
            }
            FrameType::RESOURCE_DESTROY => {
                let frame = ResourceDestroy::decode(buf)?;
                Ok(Self::ResourceDestroy(frame))
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Packet {
    pub header: Header,
    pub body: PacketBody,
}

#[derive(Clone, Debug)]
pub enum PacketBody {
    Handshake(Handshake),
    Shutdown(Shutdown),
    Ack(Ack),
    AckAck(AckAck),
    Nak(Nak),
    Data(Vec<u8>),
}

impl PacketBody {
    #[inline]
    pub fn packet_type(&self) -> PacketType {
        match self {
            Self::Handshake(_) => PacketType::HANDSHAKE,
            Self::Shutdown(_) => PacketType::SHUTDOWN,
            Self::Ack(_) => PacketType::ACK,
            Self::AckAck(_) => PacketType::ACKACK,
            Self::Nak(_) => PacketType::NAK,
            Self::Data(_) => PacketType::DATA,
        }
    }

    #[inline]
    pub fn as_data(&self) -> Option<&[u8]> {
        match self {
            Self::Data(buf) => Some(buf),
            _ => None,
        }
    }
}

impl From<Handshake> for PacketBody {
    #[inline]
    fn from(value: Handshake) -> Self {
        Self::Handshake(value)
    }
}

impl From<Shutdown> for PacketBody {
    #[inline]
    fn from(value: Shutdown) -> Self {
        Self::Shutdown(value)
    }
}

impl From<Ack> for PacketBody {
    #[inline]
    fn from(value: Ack) -> Self {
        Self::Ack(value)
    }
}

impl From<AckAck> for PacketBody {
    #[inline]
    fn from(value: AckAck) -> Self {
        Self::AckAck(value)
    }
}

impl From<Nak> for PacketBody {
    #[inline]
    fn from(value: Nak) -> Self {
        Self::Nak(value)
    }
}

impl Encode for Packet {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        let mut header = self.header;

        match &self.body {
            PacketBody::Handshake(body) => {
                header.packet_type = PacketType::HANDSHAKE;
                header.encode(&mut buf)?;

                body.encode(&mut buf)?;
            }
            PacketBody::Shutdown(body) => {
                header.packet_type = PacketType::SHUTDOWN;
                header.encode(&mut buf)?;

                body.encode(&mut buf)?;
            }
            PacketBody::Ack(body) => {
                header.packet_type = PacketType::ACK;
                header.encode(&mut buf)?;

                body.encode(&mut buf)?;
            }
            PacketBody::AckAck(body) => {
                header.packet_type = PacketType::ACKACK;
                header.encode(&mut buf)?;

                body.encode(&mut buf)?;
            }
            PacketBody::Nak(body) => {
                header.packet_type = PacketType::NAK;
                header.encode(&mut buf)?;

                body.encode(&mut buf)?;
            }
            PacketBody::Data(body) => {
                header.packet_type = PacketType::DATA;
                header.encode(&mut buf)?;

                buf.put_slice(body);
            }
        }

        Ok(())
    }
}

impl Decode for Packet {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let header = Header::decode(&mut buf)?;

        let body = match header.packet_type {
            PacketType::DATA => {
                let mut body = vec![0; buf.remaining()];
                buf.copy_to_slice(&mut body);

                PacketBody::Data(body)
            }
            PacketType::HANDSHAKE => PacketBody::Handshake(Handshake::decode(buf)?),
            PacketType::SHUTDOWN => PacketBody::Shutdown(Shutdown::decode(buf)?),
            PacketType::ACK => PacketBody::Ack(Ack::decode(buf)?),
            PacketType::ACKACK => PacketBody::AckAck(AckAck::decode(buf)?),
            PacketType::NAK => PacketBody::Nak(Nak::decode(buf)?),
            _ => unreachable!(),
        };

        Ok(Self { header, body })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameType(u16);

impl FrameType {
    /// The `FrameType` for the [`EntityCreate`] frame.
    pub const ENTITY_CREATE: Self = Self(0);

    /// The `FrameType` for the [`EntityDestroy`] frame.
    pub const ENTITY_DESTROY: Self = Self(1);

    /// The `FrameType` for the [`EntityTranslate`] frame.
    pub const ENTITY_TRANSLATE: Self = Self(2);

    /// The `FrameType` for the [`EntityRotate`] frame.
    pub const ENTITY_ROTATE: Self = Self(3);

    pub const SPAWN_HOST: Self = Self(5);

    pub const PLAYER_JOIN: Self = Self(6);
    pub const PLAYER_LEAVE: Self = Self(7);

    pub const ENTITY_ACTION: Self = Self(0x11);

    pub const TRIGGER_ACTION: Self = Self(0x20);

    pub const PLAYER_MOVE: Self = Self(0x41);

    pub const ENTITY_COMPONENT_ADD: Self = Self(0x50);
    pub const ENTITY_COMPONENT_REMOVE: Self = Self(0x51);
    pub const ENTITY_COMPONENT_UPDATE: Self = Self(0x52);
    pub const RESOURCE_CREATE: Self = Self(0x53);
    pub const RESOURCE_DESTROY: Self = Self(0x54);
}

impl TryFrom<u16> for FrameType {
    type Error = InvalidFrameType;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match Self(value) {
            Self::ENTITY_CREATE => Ok(Self::ENTITY_CREATE),
            Self::ENTITY_DESTROY => Ok(Self::ENTITY_DESTROY),
            Self::ENTITY_TRANSLATE => Ok(Self::ENTITY_TRANSLATE),
            Self::ENTITY_ROTATE => Ok(Self::ENTITY_ROTATE),
            Self::ENTITY_ACTION => Ok(Self::ENTITY_ACTION),
            Self::ENTITY_COMPONENT_ADD => Ok(Self::ENTITY_COMPONENT_ADD),
            Self::ENTITY_COMPONENT_REMOVE => Ok(Self::ENTITY_COMPONENT_REMOVE),
            Self::ENTITY_COMPONENT_UPDATE => Ok(Self::ENTITY_COMPONENT_UPDATE),
            Self::SPAWN_HOST => Ok(Self::SPAWN_HOST),
            Self::PLAYER_JOIN => Ok(Self::PLAYER_JOIN),
            Self::PLAYER_LEAVE => Ok(Self::PLAYER_LEAVE),
            Self::PLAYER_MOVE => Ok(Self::PLAYER_MOVE),
            Self::RESOURCE_CREATE => Ok(Self::RESOURCE_CREATE),
            Self::RESOURCE_DESTROY => Ok(Self::RESOURCE_DESTROY),
            _ => Err(InvalidFrameType(value)),
        }
    }
}

impl Encode for FrameType {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for FrameType {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(Self::try_from(u16::decode(buf)?)?)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("invalid frame type: {0}")]
pub struct InvalidFrameType(pub u16);

impl From<u16> for InvalidFrameType {
    #[inline]
    fn from(value: u16) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error("invalid entity kind: {0}")]
pub struct InvalidEntityKind(pub u8);

impl From<u8> for InvalidEntityKind {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum EncryptionField {
    None,
    Aes128,
}

impl Encode for ObjectId {
    type Error = Infallible;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ObjectId {
    type Error = EofError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        RecordReference::decode(buf).map(Self)
    }
}

impl Encode for ControlFrame {
    type Error = <u16 as Encode>::Error;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ControlFrame {
    type Error = <u16 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u16::decode(buf).map(Self)
    }
}

/// An inclusive sequence range.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SequenceRange {
    pub start: Sequence,
    pub end: Sequence,
}

impl Encode for SequenceRange {
    type Error = <Sequence as Encode>::Error;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        if self.start == self.end {
            self.start.encode(buf)
        } else {
            let start = self.start.to_bits() | (1 << 31);
            let end = self.end;

            start.encode(&mut buf)?;
            end.encode(buf)
        }
    }
}

impl Decode for SequenceRange {
    type Error = <u32 as Decode>::Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut start = u32::decode(&mut buf)?;

        if start & (1 << 31) == 0 {
            Ok(Self {
                start: Sequence::from_bits(start),
                end: Sequence::from_bits(start),
            })
        } else {
            start &= (1 << 31) - 1;
            let end = u32::decode(&mut buf)?;

            Ok(Self {
                start: Sequence::from_bits(start),
                end: Sequence::from_bits(end),
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourceCreate {
    pub id: ServerResource,
    pub data: Vec<u8>,
}

impl Encode for ResourceCreate {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.id.encode(&mut buf)?;
        VarInt(self.data.len() as u64).encode(&mut buf)?;
        buf.put_slice(&self.data);
        Ok(())
    }
}

impl Decode for ResourceCreate {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ServerResource::decode(&mut buf)?;
        let len = VarInt::<u64>::decode(&mut buf)?;
        let mut data = Vec::new();
        for _ in 0..len.0 {
            data.push(u8::decode(&mut buf)?);
        }
        Ok(Self { id, data })
    }
}

impl Encode for ServerResource {
    type Error = <u64 as Encode>::Error;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for ServerResource {
    type Error = <u64 as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u64::decode(buf).map(Self)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ResourceDestroy {
    pub id: ServerResource,
}

impl Encode for ResourceDestroy {
    type Error = Infallible;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.id.encode(buf)?;
        Ok(())
    }
}

impl Decode for ResourceDestroy {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let id = ServerResource::decode(buf)?;
        Ok(Self { id })
    }
}

#[cfg(test)]
mod tests {
    use game_common::world::control_frame::ControlFrame;

    use crate::proto::Flags;

    use super::sequence::Sequence;
    use super::{Decode, Encode, Header, PacketType, SequenceRange};

    #[test]
    fn header_encode_sequence() {
        let sequence: u32 = u32::MAX >> 1;

        let header = Header {
            packet_type: PacketType::DATA,
            sequence: Sequence::from_bits(sequence),
            control_frame: ControlFrame(0),
            flags: Flags::new(),
        };

        let mut buf = Vec::new();
        header.encode(&mut buf).unwrap();

        let output = [
            0b0111_1111, // Sequence 0
            0b1111_1111, // Sequence 1
            0b1111_1111, // Sequence 2
            0b1111_1111, // Sequence 3
            0b0000_0000, // ControlFrame 0
            0b0000_0000, // ControlFrame 1
            0b0000_0000, // Reserved
            0b0000_0000, // Reserved
        ];
        assert_eq!(buf, output);
    }

    #[test]
    fn header_decode_sequence() {
        let input = [
            0b0111_1111, // Sequence 0
            0b1111_1111, // Sequence 1
            0b1111_1111, // Sequence 2
            0b1111_1111, // Sequence 3
            0b0000_0000, // ControlFrame 0
            0b0000_0000, // ControlFrame 1
            0b0000_0000, // Reserved
            0b0000_0000, // Reserved
        ];

        let sequence = Sequence::from_bits(u32::MAX >> 1);

        let header = Header::decode(&input[..]).unwrap();

        assert_eq!(header.packet_type, PacketType::DATA);
        assert_eq!(header.sequence, sequence);
    }

    #[test]
    fn sequence_range_single_encode() {
        let start = Sequence::MAX;
        let end = Sequence::MAX;
        let range = SequenceRange { start, end };

        let mut buf = Vec::new();
        range.encode(&mut buf).unwrap();

        let output = [
            0b0111_1111, // Start/End
            0b1111_1111, // Start/End
            0b1111_1111, // Start/End
            0b1111_1111, // Start/End
        ];
        assert_eq!(buf, output);
    }

    #[test]
    fn sequence_range_range_encode() {
        let start = Sequence::new(0);
        let end = Sequence::MAX;
        let range = SequenceRange { start, end };

        let mut buf = Vec::new();
        range.encode(&mut buf).unwrap();

        let output = [
            0b1000_0000, // Start
            0b0000_0000, // Start
            0b0000_0000, // Start
            0b0000_0000, // Start
            0b0111_1111, // End
            0b1111_1111, // End
            0b1111_1111, // End
            0b1111_1111, // End
        ];
        assert_eq!(buf, output);
    }

    #[test]
    fn sequence_range_single_decode() {
        let input = [
            0b0111_1111, // Start/End
            0b1111_1111, // Start/End
            0b1111_1111, // Start/End
            0b1111_1111, // Start/End
        ];

        let range = SequenceRange::decode(&input[..]).unwrap();
        assert_eq!(range.start, Sequence::MAX);
        assert_eq!(range.end, Sequence::MAX);
    }

    #[test]
    fn sequence_range_range_decode() {
        let input = [
            0b1000_0000, // Start
            0b0000_0000, // Start
            0b0000_0000, // Start
            0b0000_0000, // Start
            0b0111_1111, // End
            0b1111_1111, // End
            0b1111_1111, // End
            0b1111_1111, // End
        ];

        let range = SequenceRange::decode(&input[..]).unwrap();
        assert_eq!(range.start, Sequence::new(0));
        assert_eq!(range.end, Sequence::MAX);
    }
}
