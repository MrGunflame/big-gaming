use core::mem::MaybeUninit;

use crate::components::{Decode, Encode};
use crate::entity::EntityId;
use crate::raw::{player_lookup, player_set_active, RESULT_OK};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
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
