use game_common::components::physics::{ColliderShape, Cuboid};
use game_common::entity::EntityId;
use game_common::math::Ray;
use game_physics::query::QueryFilter;
use game_tracing::trace_span;
use game_wasm::raw::physics::{CastRayResult, QueryFilter as RawQueryFilter};
use glam::{Quat, Vec3};
use wasmtime::Caller;

use crate::builtin::CallerExt;
use crate::instance::State;

pub fn physics_cast_ray(
    mut caller: Caller<'_, State<'_>>,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    max_toi: f32,
    filter_ptr: u32,
    out: u32,
) -> wasmtime::Result<u32> {
    let _span = trace_span!("physics_cast_ray").entered();
    tracing::trace!("physics_cast_ray(origin_x = {}, origin_y = {}, origin_z = {}, direction_x = {}, direction_y = {}, direction_z = {}, max_toi = {})", origin_x, origin_y, origin_z, direction_x, direction_y, direction_z, max_toi);

    let ray = Ray {
        origin: Vec3::new(origin_x, origin_y, origin_z),
        direction: Vec3::new(direction_x, direction_y, direction_z),
    };

    let filter = read_query_filter(&mut caller, filter_ptr)?;

    let (entity_id, toi) = match caller
        .data()
        .physics_pipeline
        .cast_ray(ray, max_toi, filter)
    {
        Some((entity_id, toi)) => (entity_id, toi),
        None => return Ok(1),
    };

    caller.write(
        out,
        &CastRayResult {
            entity_id: entity_id.into_raw(),
            toi,
            _pad0: 0,
        },
    )?;

    Ok(0)
}

pub fn physics_cast_shape(
    mut caller: Caller<'_, State<'_>>,
    translation_x: f32,
    translation_y: f32,
    translation_z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    rotation_w: f32,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    shape: u32,
    max_toi: f32,
    filter: u32,
    out: u32,
) -> wasmtime::Result<u32> {
    let _span = trace_span!("physics_cast_shape").entered();

    let translation = Vec3::new(translation_x, translation_y, translation_z);
    let rotation = Quat::from_xyzw(rotation_x, rotation_y, rotation_z, rotation_w);
    let direction = Vec3::new(direction_x, direction_y, direction_z);

    let shape = caller.read::<game_wasm::raw::physics::Shape>(shape)?;
    let shape = ColliderShape::Cuboid(Cuboid {
        hx: shape.hx,
        hy: shape.hy,
        hz: shape.hz,
    });
    let filter = read_query_filter(&mut caller, filter)?;

    let (entity, toi) = match caller.data().physics_pipeline.cast_shape(
        translation,
        rotation,
        direction,
        max_toi,
        shape,
        filter,
    ) {
        Some((entity, toi)) => (entity, toi),
        None => return Ok(1),
    };

    caller.write(
        out,
        &CastRayResult {
            entity_id: entity.into_raw(),
            toi,
            _pad0: 0,
        },
    )?;

    Ok(0)
}

fn read_query_filter(
    caller: &mut Caller<'_, State<'_>>,
    ptr: u32,
) -> wasmtime::Result<QueryFilter> {
    let filter: RawQueryFilter = caller.read(ptr)?;

    let mut exclude_entities = Vec::new();
    for index in 0..filter.exclude_entities_len {
        let ptr = filter.exclude_entities_ptr + (index * std::mem::size_of::<EntityId>() as u32);

        let entity = caller.read::<EntityId>(ptr)?;
        exclude_entities.push(entity);
    }

    Ok(QueryFilter { exclude_entities })
}
