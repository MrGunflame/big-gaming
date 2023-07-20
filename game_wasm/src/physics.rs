use core::mem::MaybeUninit;

use crate::entity::EntityId;
use crate::math::Ray;
use crate::raw::physics::{physics_cast_ray, CastRayResult};
use crate::raw::{PtrMut, Usize};

pub fn cast_ray(ray: Ray, max_toi: f32) -> Option<RayHit> {
    let mut out = MaybeUninit::<CastRayResult>::uninit();
    let ptr = PtrMut::from_raw(out.as_mut_ptr() as Usize);

    unsafe {
        physics_cast_ray(
            ray.origin.x,
            ray.origin.y,
            ray.origin.z,
            ray.direction.x,
            ray.direction.y,
            ray.direction.z,
            max_toi,
            ptr,
        );
    }

    let res = unsafe { out.assume_init() };

    if res.entity_id != 0 {
        Some(RayHit {
            entity: EntityId::from_raw(res.entity_id),
            toi: res.toi,
        })
    } else {
        None
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RayHit {
    pub entity: EntityId,
    pub toi: f32,
}
