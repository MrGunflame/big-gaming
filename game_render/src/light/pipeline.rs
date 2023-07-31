use std::collections::HashMap;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bytemuck::{Pod, Zeroable};
use game_common::components::transform::GlobalTransform;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::BufferUsages;

use crate::render_pass::RenderNodes;
use crate::RenderDevice;

use super::DirectionalLight;

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct DirectionalLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct PointLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

#[derive(Debug, Default, Resource)]
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
        let uniform = DirectionalLightUniform {
            position: transform.0.translation.to_array(),
            color: light.color,
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

    let lights: Vec<_> = cache.entities.values().copied().collect();

    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("directional_light_buffer"),
        contents: bytemuck::cast_slice(&lights),
        usage: BufferUsages::STORAGE,
    });

    render_nodes.directional_lights = Some(buffer);
}
