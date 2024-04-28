use core::mem::MaybeUninit;

use glam::{Quat, Vec3};

use crate::components::builtin::{Axis, ColliderShape};
use crate::entity::EntityId;
use crate::math::Ray;
use crate::raw::physics::{
    physics_cast_ray, physics_cast_shape, Ball, Capsule, CastRayResult, Cuboid, SHAPE_TYPE_BALL,
    SHAPE_TYPE_CAPSULE, SHAPE_TYPE_CUBOID,
};
use crate::raw::physics::{QueryFilter as RawQueryFilter, Shape as RawShape};
use crate::raw::RESULT_OK;

pub fn cast_ray(ray: Ray, max_toi: f32, filter: QueryFilter<'_>) -> Option<RayHit> {
    let filter = build_raw_query_filter(filter);
    let mut out = MaybeUninit::<CastRayResult>::uninit();

    let res = unsafe {
        physics_cast_ray(
            ray.origin.x,
            ray.origin.y,
            ray.origin.z,
            ray.direction.x,
            ray.direction.y,
            ray.direction.z,
            max_toi,
            &filter,
            out.as_mut_ptr(),
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

pub fn cast_shape(
    translation: Vec3,
    mut rotation: Quat,
    mut direction: Vec3,
    shape: &ColliderShape,
    max_toi: f32,
    filter: QueryFilter<'_>,
) -> Option<RayHit> {
    let filter = build_raw_query_filter(filter);

    // This is a precondition for physics_cast_shape,
    // but should we always normalize here or require
    // the caller to guarantee it?
    direction = direction.normalize_or_zero();
    rotation = rotation.normalize();

    let (shape_type, shape) = match shape {
        ColliderShape::Cuboid(cuboid) => (
            SHAPE_TYPE_CUBOID,
            RawShape {
                cuboid: Cuboid {
                    hx: cuboid.hx,
                    hy: cuboid.hy,
                    hz: cuboid.hz,
                },
            },
        ),
        ColliderShape::Ball(ball) => (
            SHAPE_TYPE_BALL,
            RawShape {
                ball: Ball {
                    radius: ball.radius,
                },
            },
        ),
        ColliderShape::Capsule(capsule) => (
            SHAPE_TYPE_CAPSULE,
            RawShape {
                capsule: Capsule {
                    axis: match capsule.axis {
                        Axis::X => 0,
                        Axis::Y => 1,
                        Axis::Z => 2,
                    },
                    half_height: capsule.half_height,
                    radius: capsule.radius,
                },
            },
        ),
    };

    let mut out = MaybeUninit::uninit();

    let res = unsafe {
        physics_cast_shape(
            translation.x,
            translation.y,
            translation.z,
            rotation.x,
            rotation.y,
            rotation.z,
            rotation.w,
            direction.x,
            direction.y,
            direction.z,
            shape_type,
            &shape,
            max_toi,
            &filter,
            out.as_mut_ptr(),
        )
    };

    if res == RESULT_OK {
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
    RawQueryFilter {
        exclude_entities_ptr: filter.exclude_entities.as_ptr(),
        exclude_entities_len: filter.exclude_entities.len(),
    }
}
