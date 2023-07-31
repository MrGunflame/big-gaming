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

    render_nodes.directional_lights = buffer;
}

#[derive(Clone, Debug)]
pub struct DirectionalLightBuffer {
    /// We need to respect the structure of the GPU buffer:
    /// ```text
    /// struct Buffer {
    ///     // `count` is aligned to `T`.
    ///     count: u32,
    ///     // At least 1 field needs to be present.
    ///     elems: array<T>,
    /// }
    /// ```
    buf: Vec<u8>,
}

impl DirectionalLightBuffer {
    pub fn new() -> Self {
        let count = Self::create_zero_count();
        let uniform = &[DirectionalLightUniform::zeroed()];
        let stub: &[u8] = bytemuck::cast_slice(uniform);

        let mut buf = Vec::with_capacity(count.len() + stub.len());
        buf.extend(count);
        buf.extend(stub);

        Self { buf }
    }

    pub fn len(&self) -> u32 {
        let bytes = &self.buf[0..4];
        u32::from_ne_bytes(bytes.try_into().unwrap())
    }

    pub fn push(&mut self, light: DirectionalLightUniform) {
        self.buf.extend(bytemuck::cast_slice(&[light]));

        let index = self.len() as usize;
        self.buf.resize(
            Self::create_zero_count().len()
                + (index + 1) * std::mem::size_of::<DirectionalLightUniform>(),
            0,
        );

        let slice = &mut self.buf[index..std::mem::size_of::<DirectionalLightUniform>()];
        slice.copy_from_slice(bytemuck::cast_slice(&[light]));
    }

    pub fn clear(&mut self) {
        self.buf
            .truncate(4 + std::mem::size_of::<DirectionalLight>());
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    const fn create_zero_count() -> &'static [u8] {
        &[0; std::mem::size_of::<DirectionalLightUniform>()]
    }
}

impl Default for DirectionalLightBuffer {
    fn default() -> Self {
        Self::new()
    }
}
