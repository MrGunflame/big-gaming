use core::mem::MaybeUninit;

use crate::encoding::{Decode, DecodeError, Encode, Primitive, Reader, Writer};
use crate::entity::EntityId;
use crate::raw::{player_lookup, player_set_active, RESULT_OK};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(u64);

impl PlayerId {
    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self(bits)
    }

    #[inline]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    pub fn set_active(&self, entity: EntityId) {
        player_set_active_safe(*self, entity);
    }
}

#[inline]
pub(crate) fn player_lookup_safe(entity: EntityId) -> Option<PlayerId> {
    let mut player_id = MaybeUninit::uninit();

    match unsafe { player_lookup(entity.into_raw(), player_id.as_mut_ptr()) } {
        RESULT_OK => {
            let player_id = unsafe { player_id.assume_init() };
            Some(PlayerId(player_id))
        }
        _ => None,
    }
}

#[inline]
pub(crate) fn player_set_active_safe(player: PlayerId, entity: EntityId) {
    match unsafe { player_set_active(player.0, entity.into_raw()) } {
        RESULT_OK => (),
        _ => unreachable!(),
    }
}

impl Encode for PlayerId {
    fn encode<W>(&self, mut writer: W)
    where
        W: Writer,
    {
        writer.write(Primitive::PlayerId, &self.0.to_le_bytes());
    }
}

impl Decode for PlayerId {
    type Error = DecodeError;

    fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
    where
        R: Reader,
    {
        if reader.next() != Some(Primitive::PlayerId) {
            return Err(DecodeError);
        }

        let bytes: [u8; 8] = reader.chunk().try_into().map_err(|_| DecodeError)?;
        Ok(PlayerId(u64::from_le_bytes(bytes)))
    }
}
