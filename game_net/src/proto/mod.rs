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

use std::convert::Infallible;

use bytes::{Buf, BufMut};
use game_common::net::ServerEntity;
use glam::{Quat, Vec3};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
#[error(transparent)]
pub enum Error {
    UnexpectedEof(#[from] EofError),
    InvalidPacketType(#[from] InvalidPacketType),
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

impl Encode for Quat {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, mut buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.x.encode(&mut buf)?;
        self.y.encode(&mut buf)?;
        self.z.encode(&mut buf)?;
        self.w.encode(&mut buf)?;
        Ok(())
    }
}

impl Decode for Quat {
    type Error = EofError;

    #[inline]
    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let x = f32::decode(&mut buf)?;
        let y = f32::decode(&mut buf)?;
        let z = f32::decode(&mut buf)?;
        let w = f32::decode(&mut buf)?;
        Ok(Self::from_xyzw(x, y, z, w))
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PacketType(u16);

impl PacketType {
    pub const HANDSHAKE: Self = Self(0);
    pub const SHUTDOWN: Self = Self(1);

    pub const ACK: Self = Self(2);
    pub const NAK: Self = Self(4);
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

#[derive(Copy, Clone, Debug)]
pub struct Header {
    pub packet_type: PacketType,
    pub timestamp: u32,
    pub sequence_number: u32,
}

/// Creates a new entity on the client.
#[derive(Copy, Clone, Debug)]
pub struct EntityCreate {
    pub entity: ServerEntity,
    pub kind: EntityKind,
    pub translation: Vec3,
    pub rotation: Quat,
}

/// Destroys a entity on the client.
#[derive(Copy, Clone, Debug)]
pub struct EntityDestroy {
    pub entity: ServerEntity,
}

/// Update the translation of an entity.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EntityTranslate {
    pub entity: ServerEntity,
    /// The new translation (absolute) translation of the entity.
    pub translation: Vec3,
}

/// Update the rotation of an entity. Contains the new rotation.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EntityRotate {
    pub entity: ServerEntity,
    /// The new rotation (absolute) of the entity.
    pub rotation: Quat,
}

#[derive(Clone, Debug)]
pub enum Frame {
    EntityCreate(EntityCreate),
    EntityDestroy(EntityDestroy),
    EntityTranslate(EntityTranslate),
    EntityRotate(EntityRotate),
}

#[derive(Clone, Debug)]
pub struct Packet {
    pub header: Header,
    pub frames: Vec<Frame>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Object,
    Actor,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameType(u16);

impl FrameType {
    pub const ENTITY_CREATE: Self = Self(0);
    pub const ENTITY_DESTROY: Self = Self(1);
    pub const ENTITY_TRANSLATE: Self = Self(2);
    pub const ENTITY_ROTATE: Self = Self(3);
}
