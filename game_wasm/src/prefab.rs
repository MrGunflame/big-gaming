use core::mem::MaybeUninit;

use crate::entity::EntityId;
use crate::raw::{prefab_spawn, RESULT_NO_RECORD, RESULT_OK};
use crate::record::RecordReference;
use crate::{unreachable_unchecked, Error, ErrorImpl};

/// Spawns a new prefab with the given `id`.
///
/// Returns the [`EntityId`] of the spawned prefab. You should attach the [`EntityId`] as a
/// children of another entity.
///
/// # Errors
///
/// Returns an [`Error`] if spawning the prefab fails because `id` does not refer to a valid
/// prefab.
pub fn spawn_prefab(id: RecordReference) -> Result<EntityId, Error> {
    let mut out = MaybeUninit::<u64>::uninit();

    match unsafe { prefab_spawn(&id, out.as_mut_ptr()) } {
        RESULT_OK => {
            let entity = unsafe { EntityId::from_raw(out.assume_init()) };
            Ok(entity)
        }
        RESULT_NO_RECORD => Err(Error(ErrorImpl::NoRecord(id))),
        _ => unsafe { unreachable_unchecked() },
    }
}
