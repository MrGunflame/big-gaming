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

use super::{DirectionalLight, PointLight, SpotLight};

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct DirectionalLightUniform {
    pub direction: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub intensity: f32,
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
    pub intensity: f32,
    pub radius: f32,
    pub _pad1: [u32; 3],
}

impl GpuBuffer for PointLightUniform {
    const SIZE: usize = std::mem::size_of::<Self>();
    const ALIGN: usize = 16;
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct SpotLightUniform {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub direction: [f32; 3],
    pub _pad1: u32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
    pub _pad2: [u32; 1],
}

impl GpuBuffer for SpotLightUniform {
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
            intensity: illuminance_to_candelas(light.illuminance),
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
            intensity: light.intensity,
            radius: light.radius,
            _pad0: 0,
            _pad1: [0; 3],
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

#[derive(Clone, Debug, Default, Resource)]
pub struct SpotLightCache {
    entities: HashMap<Entity, SpotLightUniform>,
}

pub fn update_spot_lights(
    device: Res<RenderDevice>,
    mut cache: ResMut<SpotLightCache>,
    entities: Query<(Entity, &SpotLight, &GlobalTransform)>,
    mut render_nodes: ResMut<RenderNodes>,
) {
    let mut changed = false;

    for (entity, light, transform) in &entities {
        let direction = transform.0.rotation * -Vec3::Z;

        let uniform = SpotLightUniform {
            position: transform.0.translation.to_array(),
            direction: direction.to_array(),
            inner_cutoff: light.inner_cutoff,
            outer_cutoff: light.outer_cutoff,
            color: light.color.as_rgb(),
            intensity: light.intensity,
            radius: light.radius,
            _pad0: 0,
            _pad1: 0,
            _pad2: [0; 1],
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

    let lights: DynamicBuffer<SpotLightUniform> = cache.entities.values().copied().collect();

    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("spot_light_buffer"),
        contents: lights.as_bytes(),
        usage: BufferUsages::STORAGE,
    });

    render_nodes.spot_lights = buffer;
}

fn illuminance_to_candelas(lux: f32) -> f32 {
    // FIXME: Un-harcode exposure in the future.
    // https://google.github.io/filament/Filament.html#imagingpipeline/physicallybasedcamera/exposuresettings
    let aperture = 4.0;
    let shutter_speed = 1.0 / 250.0;
    let sensitivity = 100.0;

    let ev100 = f32::log2(aperture * aperture / shutter_speed) - f32::log2(sensitivity / 100.0);
    let exposure = 1.0 / (f32::powf(2.0, ev100) * 1.2);
    lux * exposure
}
