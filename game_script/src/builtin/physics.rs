use bytemuck::{Pod, Zeroable};
use game_common::components::{Axis, Ball, Capsule, ColliderShape, Cuboid, TriMesh};
use game_common::entity::EntityId;
use game_common::math::Ray;
use game_physics::query::QueryFilter;
use game_tracing::trace_span;
use game_wasm::raw::physics::{
    CastRayResult, SHAPE_TYPE_BALL, SHAPE_TYPE_CAPSULE, SHAPE_TYPE_CUBOID, SHAPE_TYPE_TRIMESH,
};
use glam::{Quat, Vec3};
use wasmtime::Caller;

use crate::builtin::{assert_caller_precondition, log_fn_invocation, AsMemory};
use crate::instance::State;

pub fn physics_cast_ray(
    mut caller: Caller<'_, State>,
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

    let ray = Ray {
        origin: Vec3::new(origin_x, origin_y, origin_z),
        direction: Vec3::new(direction_x, direction_y, direction_z),
    };

    assert_caller_precondition!(stringify!(physics_cast_ray), ray.direction.is_normalized());

    let filter = read_query_filter(&mut caller, filter_ptr)?;

    let res = caller
        .data()
        .as_run()?
        .physics_pipeline()
        .cast_ray(ray, max_toi, &filter);

    log_fn_invocation! {
        stringify!(physics_cast_ray),
        origin_x,
        origin_y,
        origin_z,
        direction_x,
        direction_y,
        direction_z,
        max_toi,
        filter => res
    }

    match res {
        Some(hit) => {
            caller.write(
                out,
                &CastRayResult {
                    entity_id: hit.entity.into_raw(),
                    toi: hit.toi,
                    _pad0: 0,
                },
            )?;

            Ok(0)
        }
        None => Ok(1),
    }
}

pub fn physics_cast_shape(
    mut caller: Caller<'_, State>,
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
    shape_type: u32,
    shape: u32,
    max_toi: f32,
    filter: u32,
    out: u32,
) -> wasmtime::Result<u32> {
    let _span = trace_span!("physics_cast_shape").entered();

    let translation = Vec3::new(translation_x, translation_y, translation_z);
    let rotation = Quat::from_xyzw(rotation_x, rotation_y, rotation_z, rotation_w);
    let direction = Vec3::new(direction_x, direction_y, direction_z);

    assert_caller_precondition!(stringify!(physics_cast_shape), rotation.is_normalized());
    assert_caller_precondition!(stringify!(physics_cast_shape), direction.is_normalized());

    let shape = match shape_type {
        SHAPE_TYPE_CUBOID => {
            let shape = caller.read::<game_wasm::raw::physics::Cuboid>(shape)?;
            ColliderShape::Cuboid(Cuboid {
                hx: shape.hx,
                hy: shape.hy,
                hz: shape.hz,
            })
        }
        SHAPE_TYPE_BALL => {
            let shape = caller.read::<game_wasm::raw::physics::Ball>(shape)?;
            ColliderShape::Ball(Ball {
                radius: shape.radius,
            })
        }
        SHAPE_TYPE_CAPSULE => {
            let shape = caller.read::<game_wasm::raw::physics::Capsule>(shape)?;
            ColliderShape::Capsule(Capsule {
                axis: match shape.axis {
                    0 => Axis::X,
                    1 => Axis::Y,
                    2 => Axis::Z,
                    _ => return Err(wasmtime::Error::msg("invalid axis")),
                },
                half_height: shape.half_height,
                radius: shape.radius,
            })
        }
        SHAPE_TYPE_TRIMESH => {
            let shape = caller.read::<RawTriMesh>(shape)?;
            let vertices = caller
                .read_slice::<[f32; 3]>(shape.vertices_ptr, shape.vertices_len)?
                .iter()
                .map(|[x, y, z]| Vec3::new(*x, *y, *z))
                .collect();
            let indices: Vec<u32> = caller
                .read_slice(shape.indices_ptr, shape.indices_len)?
                .to_vec();

            assert_caller_precondition!(stringify!(physics_cast_shape), indices.len() % 3 == 0);

            ColliderShape::TriMesh(TriMesh::new(vertices, indices))
        }
        _ => return Err(wasmtime::Error::msg("invalid SHAPE_TYPE")),
    };

    let filter = read_query_filter(&mut caller, filter)?;

    let res = caller.data().as_run()?.physics_pipeline().cast_shape(
        translation,
        rotation,
        direction,
        max_toi,
        &shape,
        &filter,
    );

    log_fn_invocation!(
        "physics_cast_shape",
        translation_x,
        translation_y,
        translation_z,
        rotation_x,
        rotation_y,
        rotation_z,
        rotation_w,
        direction_x,
        direction_y,
        direction_z,
        shape,
        max_toi,
        filter => res
    );

    match res {
        Some(hit) => {
            caller.write(
                out,
                &CastRayResult {
                    entity_id: hit.entity.into_raw(),
                    toi: hit.toi,
                    _pad0: 0,
                },
            )?;

            Ok(0)
        }
        None => Ok(1),
    }
}

fn read_query_filter(caller: &mut Caller<'_, State>, ptr: u32) -> wasmtime::Result<QueryFilter> {
    let filter: RawQueryFilter = caller.read(ptr)?;

    let mut exclude_entities = Vec::new();
    for index in 0..filter.exclude_entities_len {
        let ptr = filter.exclude_entities_ptr + (index * size_of::<EntityId>() as u32);

        let entity = caller.read::<EntityId>(ptr)?;
        exclude_entities.push(entity);
    }

    Ok(QueryFilter { exclude_entities })
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawQueryFilter {
    exclude_entities_ptr: u32,
    exclude_entities_len: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct RawTriMesh {
    vertices_ptr: u32,
    vertices_len: u32,
    indices_ptr: u32,
    indices_len: u32,
}
