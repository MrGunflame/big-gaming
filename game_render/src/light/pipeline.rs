use std::collections::HashMap;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bytemuck::{Pod, Zeroable};
use game_common::components::transform::GlobalTransform;
use glam::Vec3;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::BufferUsages;

use crate::buffer::{DynamicBuffer, GpuBuffer};
use crate::render_pass::RenderNodes;
use crate::RenderDevice;

use super::{DirectionalLight, PointLight};

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct DirectionalLightUniform {
    pub direction: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

impl GpuBuffer for DirectionalLightUniform {
    const SIZE: usize = std::mem::size_of::<Self>();
    const ALIGN: usize = 16;
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct PointLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

impl GpuBuffer for PointLightUniform {
    const SIZE: usize = std::mem::size_of::<Self>();
    const ALIGN: usize = 16;
}

#[derive(Clone, Debug, Default, Resource)]
pub struct DirectionalLightCache {
    entities: HashMap<Entity, DirectionalLightUniform>,
}

pub fn update_directional_lights(
    device: Res<RenderDevice>,
    mut cache: ResMut<DirectionalLightCache>,
    entities: Query<(Entity, &DirectionalLight, &GlobalTransform)>,
    mut render_nodes: ResMut<RenderNodes>,
) {
    let mut changed = false;

    for (entity, light, transform) in &entities {
        let direction = transform.0.rotation * -Vec3::Z;

        let uniform = DirectionalLightUniform {
            direction: direction.to_array(),
            color: light.color.as_rgb(),
            _pad0: 0,
            _pad1: 0,
        };

        if let Some(light) = cache.entities.get(&entity) {
            if *light == uniform {
                continue;
            }
        }

        cache.entities.insert(entity, uniform);
        changed = true;
    }

    if !changed {
        return;
    }

    let lights: DynamicBuffer<DirectionalLightUniform> = cache.entities.values().copied().collect();

    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("directional_light_buffer"),
        contents: lights.as_bytes(),
        usage: BufferUsages::STORAGE,
    });

    render_nodes.directional_lights = buffer;
}

#[derive(Clone, Debug, Default, Resource)]
pub struct PointLightCache {
    entities: HashMap<Entity, PointLightUniform>,
}

pub fn update_point_lights(
    device: Res<RenderDevice>,
    mut cache: ResMut<PointLightCache>,
    entities: Query<(Entity, &PointLight, &GlobalTransform)>,
    mut render_nodes: ResMut<RenderNodes>,
) {
    let mut changed = false;

    for (entity, light, transform) in &entities {
        let uniform = PointLightUniform {
            position: transform.0.translation.to_array(),
            color: light.color.as_rgb(),
            _pad0: 0,
            _pad1: 0,
        };

        if let Some(light) = cache.entities.get(&entity) {
            if *light == uniform {
                continue;
            }
        }

        cache.entities.insert(entity, uniform);
        changed = true;
    }

    if !changed {
        return;
    }

    let lights: DynamicBuffer<PointLightUniform> = cache.entities.values().copied().collect();

    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("point_light_buffer"),
        contents: lights.as_bytes(),
        usage: BufferUsages::STORAGE,
    });

    render_nodes.point_lights = buffer;
}
