use core::mem::MaybeUninit;

use crate::entity::EntityId;
use crate::math::Ray;
use crate::raw::physics::QueryFilter as RawQueryFilter;
use crate::raw::physics::{physics_cast_ray, CastRayResult};
use crate::raw::{Ptr, PtrMut, Usize};

pub fn cast_ray(ray: Ray, max_toi: f32, filter: QueryFilter<'_>) -> Option<RayHit> {
    let filter = build_raw_query_filter(filter);
    let filter_ptr = Ptr::from_raw(&filter as *const RawQueryFilter as Usize);

    let mut out = MaybeUninit::<CastRayResult>::uninit();
    let ptr = PtrMut::from_raw(out.as_mut_ptr() as Usize);

    let res = unsafe {
        physics_cast_ray(
            ray.origin.x,
            ray.origin.y,
            ray.origin.z,
            ray.direction.x,
            ray.direction.y,
            ray.direction.z,
            max_toi,
            filter_ptr,
            ptr,
        )
    };

    if res == 0 {
        let res = unsafe { out.assume_init() };
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

#[derive(Clone, Debug, Default)]
pub struct QueryFilter<'a> {
    pub exclude_entities: &'a [EntityId],
}

fn build_raw_query_filter(filter: QueryFilter<'_>) -> RawQueryFilter {
    let exclude_entities_ptr = filter.exclude_entities.as_ptr() as Usize;
    let exclude_entities_len = filter.exclude_entities.len() as Usize;

    RawQueryFilter {
        exclude_entities_ptr,
        exclude_entities_len,
    }
}
