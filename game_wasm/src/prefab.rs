use core::mem::MaybeUninit;

use crate::entity::EntityId;
use crate::raw::{prefab_spawn, RESULT_NO_RECORD, RESULT_OK};
use crate::record::RecordReference;
use crate::{unreachable_unchecked, Error, ErrorImpl};

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
