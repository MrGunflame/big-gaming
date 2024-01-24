use bytemuck::{Pod, Zeroable};

use crate::encoding::{Decode, DecodeError, Encode, Primitive, Reader, Writer};

/// A unique identifier for an [`Entity`].
///
/// [`Entity`]: crate::world::Entity
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct EntityId(u64);

impl EntityId {
    /// Creates a `EntityId` using the specified `bits`.
    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns the underlying bits of the `EntityId`.
    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> &u64 {
        &self.0
    }
}

impl Encode for EntityId {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        writer.write(Primitive::EntityId, &self.0.to_le_bytes());
    }
}

impl Decode for EntityId {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        // if reader.next() != Some(Primitive::EntityId) {
        //     return Err(DecodeError);
        // }

        let bytes: [u8; 8] = reader.chunk().try_into().map_err(|_| DecodeError)?;
        Ok(Self(u64::from_be_bytes(bytes)))
    }
}
