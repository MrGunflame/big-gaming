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

    pub const fn dangling() -> Self {
        Self(0)
    }
}

impl Encode for EntityId {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        let bytes = self.0.to_le_bytes();
        writer.write(Primitive::EntityId, &bytes);
    }
}

impl Decode for EntityId {
    type Error = DecodeError;

    fn decode<R>(reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        // if reader.next() != Some(Primitive::EntityId) {
        //     return Err(DecodeError);
        // }

        let bytes = <[u8; 8]>::decode(reader)?;
        Ok(Self(u64::from_le_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use crate::encoding::{BinaryReader, BinaryWriter, Decode, Primitive};

    use super::EntityId;

    #[test]
    fn entity_encode_and_decode() {
        let entity = EntityId::from_raw(1288025431908348);

        let (fields, data) = BinaryWriter::new().encoded(&entity);
        assert_eq!(fields[0].primitive, Primitive::EntityId);

        let reader = BinaryReader::new(data, fields.into());
        let other = EntityId::decode(reader).unwrap();
        assert_eq!(entity, other);
    }
}
