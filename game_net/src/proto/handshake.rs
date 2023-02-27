//!
//!
//! # Handshake process
//!
//! Client: HELLO => Server
//! Server: HELLO => Client
//! Client: AGREEMENT => Server
//! Server: AGREEMENT/REJECT => Client

use std::convert::Infallible;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use bytes::{Buf, BufMut};

use super::{Decode, Encode, Error};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Handshake {
    pub version: u16,
    pub kind: HandshakeType,
    pub flags: HandshakeFlags,
    pub mtu: u16,
    pub flow_window: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, thiserror::Error)]
#[error("invalid encryption field: {0}")]
pub struct InvalidEncryptionField(pub u8);

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub enum EncryptionField {
//     None,
//     Aes128,
// }

// impl Encode for EncryptionField {
//     type Error = Infallible;

//     fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
//     where
//         B: BufMut,
//     {
//         let v = match self {
//             Self::None => 0u8,
//             Self::Aes128 => 2u8,
//         };

//         v.encode(buf)
//     }
// }

// impl Decode for EncryptionField {
//     type Error = Error;

//     fn decode<B>(buf: B) -> Result<Self, Self::Error>
//     where
//         B: Buf,
//     {
//     }
// }

///
///
/// | Code | Name      | Description |
/// | ---- | --------- | ----------- |
/// | 0    | HELLO     | |
/// | 1    | AGREEMENT | |
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HandshakeType(u8);

impl HandshakeType {
    pub const HELLO: Self = Self(0);
    pub const AGREEMENT: Self = Self(1);

    pub const REJ_UNKNOWN: Self = Self(16);
    pub const REJ_ROGUE: Self = Self(17);

    /// The advertised MTU is too low for the peer.
    pub const REJ_MTU: Self = Self(18);
}

impl Encode for HandshakeType {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for HandshakeType {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(Self::try_from(u8::decode(buf)?)?)
    }
}

impl TryFrom<u8> for HandshakeType {
    type Error = InvalidHandshakeType;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match Self(value) {
            Self::HELLO => Ok(Self::HELLO),
            Self::AGREEMENT => Ok(Self::AGREEMENT),
            Self::REJ_UNKNOWN => Ok(Self::REJ_UNKNOWN),
            Self::REJ_ROGUE => Ok(Self::REJ_ROGUE),
            Self::REJ_MTU => Ok(Self::REJ_MTU),
            _ => Err(InvalidHandshakeType(value)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, thiserror::Error)]
#[error("invalid handshake type: {0}")]
pub struct InvalidHandshakeType(pub u8);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, thiserror::Error)]
#[error("invalid handshake flags: {0:#b}")]
pub struct InvalidHandshakeFlags(pub u8);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HandshakeFlags(u8);

impl HandshakeFlags {
    pub const NONE: Self = Self(0);

    pub const SESSION: Self = Self(1);
}

impl Encode for HandshakeFlags {
    type Error = Infallible;

    #[inline]
    fn encode<B>(&self, buf: B) -> Result<(), Self::Error>
    where
        B: BufMut,
    {
        self.0.encode(buf)
    }
}

impl Decode for HandshakeFlags {
    type Error = Error;

    #[inline]
    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(Self::try_from(u8::decode(buf)?)?)
    }
}

impl BitAnd for HandshakeFlags {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for HandshakeFlags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for HandshakeFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for HandshakeFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl TryFrom<u8> for HandshakeFlags {
    type Error = InvalidHandshakeFlags;

    fn try_from(mut value: u8) -> Result<Self, Self::Error> {
        let mut flags = Self::NONE;

        if Self(value) & Self::SESSION != Self::NONE {
            // Remove the bit
            value &= u8::MAX - Self::SESSION.0;

            flags |= Self::SESSION;
        }

        if value == 0 {
            Ok(flags)
        } else {
            Err(InvalidHandshakeFlags(value))
        }
    }
}

#[cfg(test)]
mod tests {}
