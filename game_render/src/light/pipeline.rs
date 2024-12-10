use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, Device};

use crate::buffer::{DynamicBuffer, GpuBuffer};

use super::{DirectionalLight, PointLight, SpotLight};

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct DirectionalLightUniform {
    pub direction: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub intensity: f32,
}

impl GpuBuffer for DirectionalLightUniform {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

impl From<DirectionalLight> for DirectionalLightUniform {
    fn from(entity: DirectionalLight) -> Self {
        Self {
            direction: (entity.transform.rotation * -Vec3::Z).to_array(),
            color: entity.color.as_rgb(),
            intensity: illuminance_to_candelas(entity.illuminance),
            _pad0: 0,
        }
    }
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
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

impl From<PointLight> for PointLightUniform {
    fn from(entity: PointLight) -> Self {
        Self {
            position: entity.transform.translation.to_array(),
            color: entity.color.as_rgb(),
            intensity: entity.intensity,
            radius: entity.radius,
            _pad0: 0,
            _pad1: [0; 3],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct SpotLightUniform {
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
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = 16;
}

impl From<SpotLight> for SpotLightUniform {
    fn from(entity: SpotLight) -> Self {
        Self {
            position: entity.transform.translation.to_array(),
            direction: (entity.transform.rotation * -Vec3::Z).to_array(),
            color: entity.color.as_rgb(),
            intensity: entity.intensity,
            radius: entity.radius,
            inner_cutoff: entity.inner_cutoff,
            outer_cutoff: entity.outer_cutoff,
            _pad0: 0,
            _pad1: 0,
            _pad2: [0; 1],
        }
    }
}

pub fn update_directional_lights(
    device: &Device,
    entities: impl Iterator<Item = DirectionalLight>,
) -> Buffer {
    let mut lights = DynamicBuffer::new();

    for entity in entities {
        let direction = entity.transform.rotation * -Vec3::Z;

        let uniform = DirectionalLightUniform {
            direction: direction.to_array(),
            color: entity.color.as_rgb(),
            intensity: illuminance_to_candelas(entity.illuminance),
            _pad0: 0,
        };

        lights.push(uniform);
    }

    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("directional_light_buffer"),
        contents: lights.as_bytes(),
        usage: BufferUsages::STORAGE,
    })
}

pub fn update_point_lights(device: &Device, entities: impl Iterator<Item = PointLight>) -> Buffer {
    let mut lights = DynamicBuffer::new();

    for entity in entities {
        let uniform = PointLightUniform {
            position: entity.transform.translation.to_array(),
            color: entity.color.as_rgb(),
            intensity: entity.intensity,
            radius: entity.radius,
            _pad0: 0,
            _pad1: [0; 3],
        };

        lights.push(uniform);
    }

    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("point_light_buffer"),
        contents: lights.as_bytes(),
        usage: BufferUsages::STORAGE,
    })
}

pub fn update_spot_lights(device: &Device, entities: impl Iterator<Item = SpotLight>) -> Buffer {
    let mut lights = DynamicBuffer::new();

    for entity in entities {
        let direction = entity.transform.rotation * -Vec3::Z;

        let uniform = SpotLightUniform {
            direction: direction.to_array(),
            position: entity.transform.translation.to_array(),
            color: entity.color.as_rgb(),
            intensity: entity.intensity,
            radius: entity.radius,
            inner_cutoff: entity.inner_cutoff,
            outer_cutoff: entity.outer_cutoff,
            _pad0: 0,
            _pad1: 0,
            _pad2: [0; 1],
        };

        lights.push(uniform);
    }

    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("spot_light_buffer"),
        contents: lights.as_bytes(),
        usage: BufferUsages::STORAGE,
    })
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
