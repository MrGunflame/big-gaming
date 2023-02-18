//! The client/server protocol
//!
//! # Flow control
//!
//!

pub struct Frame {}

pub struct ActorSpawn {}

#[derive(Clone, Debug)]
pub struct EntityMove {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Header {
    packet_type: PacketType,
    timestamp: u32,
    sequence_number: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Handshake {
    pub version: u8,
    /// Maximum transmission unit
    pub mtu: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Shutdown {}

pub struct HandshakeState(u8);

impl HandshakeState {
    pub const CONNECT: Self = Self(1);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PacketType(u16);

impl PacketType {
    pub const HANDSHAKE: Self = Self(0);
    pub const SHUTDOWN: Self = Self(1);
}
