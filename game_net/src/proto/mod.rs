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
pub mod handshake;
pub mod sequence;
pub mod shutdown;
pub mod timestamp;

mod action;
mod combat;
mod inventory;
mod properties;
mod quat;
mod record;

use game_common::components::actions::ActionId;
use game_common::components::combat::Health;
use game_common::components::object::ObjectId;
use game_common::components::race::RaceId;
use game_common::entity::EntityId;
use game_common::id::WeakId;
use game_common::world::entity::{Actor, EntityBody, Object};
use game_common::world::terrain::{Heightmap, TerrainMesh};
use game_common::world::CellId;
pub use game_macros::{net__decode as Decode, net__encode as Encode};

use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::net::ServerEntity;
use glam::{Quat, UVec2, Vec3};
use thiserror::Error;

use self::ack::{Ack, Nak};
use self::handshake::{Handshake, InvalidHandshakeFlags, InvalidHandshakeType};
use self::sequence::Sequence;
use self::shutdown::Shutdown;
use self::timestamp::Timestamp;

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
        self.0.encode(buf)
    }
}

impl Decode for ServerEntity {
    type Error = EofError;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u64::decode(buf).map(Self)
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
/// | Timestamp                                                     |
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
/// | Timestamp                                                     |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Header {
    pub packet_type: PacketType,
    pub sequence: Sequence,
    pub timestamp: Timestamp,
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
        self.timestamp.encode(&mut buf)?;
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
        let timestamp = Timestamp::decode(&mut buf)?;

        let packet_type;
        let sequence;
        // DATA
        if word0 & 1 << 31 == 0 {
            packet_type = PacketType::DATA;
            sequence = Sequence::from_bits(word0 >> 1);

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
            timestamp,
        })
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

                terrain.cell.encode(&mut buf)?;

                terrain.height().size().encode(&mut buf)?;
                for node in terrain.height().nodes() {
                    node.encode(&mut buf)?;
                }
            }
            Self::Object(object) => {
                1u8.encode(&mut buf)?;
                object.id.encode(&mut buf)?;
            }
            Self::Actor(actor) => {
                2u8.encode(&mut buf)?;
                actor.race.encode(&mut buf)?;
                actor.health.encode(&mut buf)?;
            }
            Self::Item(item) => {
                3u8.encode(&mut buf)?;
                todo!()
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
                let cell = CellId::decode(&mut buf)?;

                let size = UVec2::decode(&mut buf)?;
                let len = (size.x as usize)
                    .checked_mul(size.y as usize)
                    .expect("terrain heightmap size overflow while decoding");

                let mut nodes = Vec::with_capacity(len);

                for _ in 0..len {
                    let node = f32::decode(&mut buf)?;
                    nodes.push(node);
                }

                Ok(Self::Terrain(TerrainMesh::new(
                    cell,
                    Heightmap::from_vec(size, nodes),
                )))
            }
            1u8 => {
                let id = ObjectId::decode(&mut buf)?;

                Ok(Self::Object(Object { id }))
            }
            2u8 => {
                let race = RaceId::decode(&mut buf)?;
                let health = Health::decode(&mut buf)?;

                Ok(Self::Actor(Actor { race, health }))
            }
            3u8 => {
                todo!()
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
        WeakId::decode(buf).map(Self)
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

/// Updates the velocity of an entity. Contains the absolute velocity.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityVelocity {
    pub entity: ServerEntity,
    /// The linear velocity of the entity.
    pub linvel: Vec3,
    /// The angular velocity of the entity.
    pub angvel: Vec3,
}

/// Updates the health of an entity.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityHealth {
    pub entity: ServerEntity,
    pub health: Health,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityAction {
    pub entity: ServerEntity,
    pub action: ActionId,
}

/// Sets the host actor host used by the client.
///
/// This is the actor the client is allowed to control.
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SpawnHost {
    pub entity: ServerEntity,
}

#[derive(Clone, Debug)]
pub struct Terrain {
    pub cell: CellId,
    pub height: Heightmap,
}

impl Encode for Terrain {
    type Error = Infallible;

    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.cell.encode(&mut buf)?;

        self.height.size().encode(&mut buf)?;

        for p in self.height.nodes() {
            p.encode(&mut buf)?;
        }

        Ok(())
    }
}

impl Decode for Terrain {
    type Error = EofError;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let cell = CellId::decode(&mut buf)?;

        let size = UVec2::decode(&mut buf)?;
        let len = (size.x as usize)
            .checked_mul(size.y as usize)
            .expect("received terrain mesh size overflowed");

        let mut nodes = Vec::with_capacity(len);

        for _ in 0..len {
            let node = f32::decode(&mut buf)?;
            nodes.push(node);
        }

        Ok(Self {
            cell,
            height: Heightmap::from_vec(size, nodes),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Frame {
    EntityCreate(EntityCreate),
    EntityDestroy(EntityDestroy),
    EntityTranslate(EntityTranslate),
    EntityRotate(EntityRotate),
    EntityVelocity(EntityVelocity),
    EntityHealth(EntityHealth),
    EntityAction(EntityAction),
    SpawnHost(SpawnHost),
}

impl Frame {
    pub fn id(&self) -> ServerEntity {
        match self {
            Self::EntityCreate(frame) => frame.entity,
            Self::EntityDestroy(frame) => frame.entity,
            Self::EntityTranslate(frame) => frame.entity,
            Self::EntityRotate(frame) => frame.entity,
            Self::EntityVelocity(frame) => frame.entity,
            Self::EntityHealth(frame) => frame.entity,
            Self::EntityAction(frame) => frame.entity,
            Self::SpawnHost(frame) => frame.entity,
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
            Self::EntityCreate(frame) => {
                FrameType::ENTITY_CREATE.encode(&mut buf)?;
                frame.encode(buf)
            }
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
            Self::EntityVelocity(frame) => {
                FrameType::ENTITY_VELOCITY.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityHealth(frame) => {
                FrameType::ENTITY_HEALTH.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::EntityAction(frame) => {
                FrameType::ENTITY_ACTION.encode(&mut buf)?;
                frame.encode(buf)
            }
            Self::SpawnHost(frame) => {
                FrameType::SPAWN_HOST.encode(&mut buf)?;
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
            FrameType::ENTITY_CREATE => {
                let frame = EntityCreate::decode(buf)?;
                Ok(Self::EntityCreate(frame))
            }
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
            FrameType::ENTITY_VELOCITY => {
                let frame = EntityVelocity::decode(buf)?;
                Ok(Self::EntityVelocity(frame))
            }
            FrameType::ENTITY_HEALTH => {
                let frame = EntityHealth::decode(buf)?;
                Ok(Self::EntityHealth(frame))
            }
            FrameType::ENTITY_ACTION => {
                let frame = EntityAction::decode(buf)?;
                Ok(Self::EntityAction(frame))
            }
            FrameType::SPAWN_HOST => {
                let frame = SpawnHost::decode(buf)?;
                Ok(Self::SpawnHost(frame))
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
    Nak(Nak),
    Frames(Vec<Frame>),
}

impl PacketBody {
    #[inline]
    pub fn packet_type(&self) -> PacketType {
        match self {
            Self::Handshake(_) => PacketType::HANDSHAKE,
            Self::Shutdown(_) => PacketType::SHUTDOWN,
            Self::Ack(_) => PacketType::ACK,
            Self::Nak(_) => PacketType::NAK,
            Self::Frames(_) => PacketType::DATA,
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
            PacketBody::Nak(body) => {
                header.packet_type = PacketType::NAK;
                header.encode(&mut buf)?;

                body.encode(&mut buf)?;
            }
            PacketBody::Frames(body) => {
                header.packet_type = PacketType::DATA;
                header.encode(&mut buf)?;

                for frame in body {
                    frame.encode(&mut buf)?;
                }
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
                let mut frames = Vec::new();
                while buf.remaining() > 0 {
                    frames.push(Frame::decode(&mut buf)?);
                }

                PacketBody::Frames(frames)
            }
            PacketType::HANDSHAKE => PacketBody::Handshake(Handshake::decode(buf)?),
            PacketType::SHUTDOWN => PacketBody::Shutdown(Shutdown::decode(buf)?),
            PacketType::ACK => PacketBody::Ack(Ack::decode(buf)?),
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

    /// The `FrameType` for the [`EntityVelocity`] frame.
    pub const ENTITY_VELOCITY: Self = Self(4);

    pub const SPAWN_HOST: Self = Self(5);

    pub const PLAYER_JOIN: Self = Self(6);
    pub const PLAYER_LEAVE: Self = Self(7);

    /// The `FrameType` for the [`EntityHealth`] frame.
    pub const ENTITY_HEALTH: Self = Self(0x10);

    pub const ENTITY_ACTION: Self = Self(0x11);

    pub const TRIGGER_ACTION: Self = Self(0x20);
}

impl TryFrom<u16> for FrameType {
    type Error = InvalidFrameType;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match Self(value) {
            Self::ENTITY_CREATE => Ok(Self::ENTITY_CREATE),
            Self::ENTITY_DESTROY => Ok(Self::ENTITY_DESTROY),
            Self::ENTITY_TRANSLATE => Ok(Self::ENTITY_TRANSLATE),
            Self::ENTITY_ROTATE => Ok(Self::ENTITY_ROTATE),
            Self::ENTITY_VELOCITY => Ok(Self::ENTITY_VELOCITY),
            Self::ENTITY_HEALTH => Ok(Self::ENTITY_HEALTH),
            Self::ENTITY_ACTION => Ok(Self::ENTITY_ACTION),
            Self::SPAWN_HOST => Ok(Self::SPAWN_HOST),
            Self::PLAYER_JOIN => Ok(Self::PLAYER_JOIN),
            Self::PLAYER_LEAVE => Ok(Self::PLAYER_LEAVE),
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

impl<T> Encode for WeakId<T>
where
    T: Encode,
{
    type Error = <T as Encode>::Error;

    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl<T> Decode for WeakId<T>
where
    T: Decode,
{
    type Error = <T as Decode>::Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        T::decode(buf).map(Self)
    }
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
        WeakId::decode(buf).map(Self)
    }
}
